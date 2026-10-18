use std::thread;
use std::time::Instant;

use crate::backend::test_support::{FakeBackend, Fault};

use super::tests::wait_for_event;
use super::*;

const ADDRESS: &str = "aa:bb:cc:dd:ee:ff";
const SERVICE: &str = "0000180d-0000-1000-8000-00805f9b34fb";
const CHARACTERISTIC: &str = "00002a37-0000-1000-8000-00805f9b34fb";

pub(super) fn runtime() -> (Arc<FakeBackend>, CoreRuntime, CoreClient) {
    let backend = Arc::new(FakeBackend::default());
    backend.add_connected_device(ADDRESS);
    let runtime = CoreRuntime::spawn_with_options(
        backend.clone(),
        CoreOptions {
            operation_timeout: Duration::from_millis(100),
            shutdown_timeout: Duration::from_millis(80),
        },
    );
    let client = runtime.client();
    (backend, runtime, client)
}

pub(super) fn success(runtime: &mut CoreRuntime, submission: Submission) -> OperationId {
    let id = submission.unwrap();
    let event = wait_for_event(runtime, |event| event.operation_id == id && event.terminal);
    assert_eq!(event.phase, Phase::Succeeded, "{event:?}");
    id
}

fn wait_pumps(backend: &FakeBackend, expected: usize) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while backend.notification_pumps_started() < expected {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn local_rejections_never_compete_with_the_event_queue() {
    let (_, mut runtime, client) = runtime();
    let mut ids = HashSet::new();
    for index in 0..(EVENT_CHANNEL_CAPACITY * 2) {
        let result = match index % 3 {
            0 => client.connect(" "),
            1 => client.read(ADDRESS, "bad-service", CHARACTERISTIC),
            _ => client.reject(
                Operation::Scan,
                EventContext::default(),
                BleError::InvalidArgument("negative timeout".to_string()),
            ),
        };
        let event = result.expect_err("rejection is returned directly");
        assert!(event.terminal);
        assert!(event.operation_id.get() > 0);
        assert!(ids.insert(event.operation_id));
    }
    assert!(runtime.try_recv().is_none());
}

#[test]
fn accepted_and_rejected_requests_each_finish_once_under_pressure() {
    let backend = Arc::new(FakeBackend::default());
    backend.add_connected_device(ADDRESS);
    let mut runtime = CoreRuntime::spawn_with_backend(backend.clone());
    let client = runtime.client();
    success(&mut runtime, client.initialize());
    backend.block_next_connect();
    let connect = client.connect(ADDRESS).unwrap();
    wait_for_event(&mut runtime, |event| {
        event.operation_id == connect && event.phase == Phase::Started
    });
    let mut expected = HashSet::from([connect]);
    let mut completed = HashSet::new();
    for _ in 0..4000 {
        match client.read(ADDRESS, SERVICE, CHARACTERISTIC) {
            Ok(id) => {
                assert!(expected.insert(id));
            }
            Err(event) => {
                assert!(expected.insert(event.operation_id));
                assert!(completed.insert(event.operation_id));
                assert!(event.terminal);
                assert_eq!(event.error.unwrap().code, "QUEUE_FULL");
            }
        }
    }
    backend.release_connect();
    let deadline = Instant::now() + Duration::from_secs(3);
    while completed.len() < expected.len() {
        while let Some(event) = runtime.try_recv() {
            if event.terminal {
                assert!(expected.contains(&event.operation_id));
                assert!(
                    completed.insert(event.operation_id),
                    "duplicate terminal event"
                );
            }
        }
        assert!(Instant::now() < deadline, "lost a terminal event");
        thread::sleep(Duration::from_millis(1));
    }
}

fn device_request(client: &CoreClient, operation: &str) -> Submission {
    match operation {
        "connect" => client.connect(ADDRESS),
        "disconnect" => client.disconnect(ADDRESS),
        "discover_services" => client.discover_services(ADDRESS),
        "read" => client.read(ADDRESS, SERVICE, CHARACTERISTIC),
        "write" => client.write(ADDRESS, SERVICE, CHARACTERISTIC, vec![1], true),
        "subscribe" => client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC),
        "unsubscribe" => client.unsubscribe(ADDRESS, SERVICE, CHARACTERISTIC),
        _ => unreachable!(),
    }
}

#[test]
fn stalled_device_operations_time_out_and_leave_the_queue_usable() {
    for operation in [
        "connect",
        "disconnect",
        "discover_services",
        "read",
        "write",
        "subscribe",
        "unsubscribe",
    ] {
        let (backend, mut runtime, client) = runtime();
        success(&mut runtime, client.initialize());
        if operation != "connect" {
            success(&mut runtime, client.connect(ADDRESS));
        }
        if operation == "unsubscribe" {
            success(
                &mut runtime,
                client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC),
            );
        }
        backend.fault_next(operation, Fault::Hang);
        let id = device_request(&client, operation).unwrap();
        let failed = wait_for_event(&mut runtime, |event| {
            event.operation_id == id && event.terminal
        });
        assert_eq!(failed.error.unwrap().code, "TIMEOUT", "{operation}");
        success(&mut runtime, device_request(&client, operation));
    }
}

#[test]
fn backend_panics_finish_requests_and_do_not_strand_the_device() {
    let (backend, mut runtime, client) = runtime();
    success(&mut runtime, client.initialize());
    backend.fault_next("connect", Fault::Panic);
    let id = client.connect(ADDRESS).unwrap();
    let failed = wait_for_event(&mut runtime, |event| {
        event.operation_id == id && event.terminal
    });
    assert_eq!(failed.error.unwrap().code, "INTERNAL_ERROR");
    success(&mut runtime, client.connect(ADDRESS));
}

#[test]
fn initialize_and_scan_backend_calls_have_deadlines() {
    let (backend, mut runtime, client) = runtime();
    backend.fault_next("initialize", Fault::Hang);
    let id = client.initialize().unwrap();
    let failed = wait_for_event(&mut runtime, |event| {
        event.operation_id == id && event.terminal
    });
    assert_eq!(failed.error.unwrap().code, "TIMEOUT");
    success(&mut runtime, client.initialize());
    backend.fault_next("start_scan", Fault::Hang);
    let id = client.start_scan(None).unwrap();
    let failed = wait_for_event(&mut runtime, |event| {
        event.operation_id == id && event.terminal
    });
    assert_eq!(failed.error.unwrap().code, "TIMEOUT");
    let id = client.start_scan(None).unwrap();
    wait_for_event(&mut runtime, |event| {
        event.operation_id == id && event.phase == Phase::Started
    });
    backend.fault_next("stop_scan", Fault::Hang);
    assert_eq!(client.stop_scan(), id);
    let failed = wait_for_event(&mut runtime, |event| {
        event.operation_id == id && event.terminal
    });
    assert_eq!(failed.error.unwrap().code, "TIMEOUT");
    let next = client.start_scan(Some(Duration::from_millis(5))).unwrap();
    let ended = wait_for_event(&mut runtime, |event| {
        event.operation_id == next && event.terminal
    });
    assert_eq!(ended.phase, Phase::Succeeded);
}

#[test]
fn shutdown_has_a_total_budget_when_cleanup_never_finishes() {
    let (backend, mut runtime, client) = runtime();
    success(&mut runtime, client.initialize());
    success(&mut runtime, client.connect(ADDRESS));
    let scan = client.start_scan(None).unwrap();
    wait_for_event(&mut runtime, |event| {
        event.operation_id == scan && event.phase == Phase::Started
    });
    backend.fault_next("stop_scan", Fault::Hang);
    backend.fault_next("disconnect", Fault::Hang);
    let started = Instant::now();
    runtime.shutdown();
    assert!(started.elapsed() < Duration::from_secs(1));
    let rejected = client.connect(ADDRESS).unwrap_err();
    assert_eq!(rejected.error.unwrap().code, "CANCELLED");
}

#[test]
fn an_ended_adapter_stream_invalidates_sessions_and_allows_reinitialization() {
    let (backend, mut runtime, client) = runtime();
    success(&mut runtime, client.initialize());
    success(&mut runtime, client.connect(ADDRESS));
    let scan = client.start_scan(None).unwrap();
    wait_for_event(&mut runtime, |event| {
        event.operation_id == scan && event.phase == Phase::Started
    });
    backend.end_adapter_stream();
    let ended = wait_for_event(&mut runtime, |event| {
        event.operation == Operation::Initialize
    });
    assert_eq!(ended.operation_id, OperationId::UNSOLICITED);
    assert!(!ended.terminal);
    assert_eq!(ended.phase, Phase::Failed);
    let cancelled = wait_for_event(&mut runtime, |event| {
        event.operation_id == scan && event.terminal
    });
    assert_eq!(cancelled.phase, Phase::Cancelled);
    let disconnected = wait_for_event(&mut runtime, |event| {
        event.operation == Operation::Disconnect
    });
    assert_eq!(
        disconnected.connection_state,
        Some(ConnectionState::Disconnected)
    );
    assert_eq!(backend.stop_scan_calls(), 1);
    assert!(!backend.is_connected(ADDRESS));
    success(&mut runtime, client.initialize());
    success(&mut runtime, client.connect(ADDRESS));
}

#[test]
fn adapter_recovery_is_bounded_when_backend_cleanup_hangs() {
    let (backend, mut runtime, client) = runtime();
    success(&mut runtime, client.initialize());
    success(&mut runtime, client.connect(ADDRESS));
    let scan = client.start_scan(None).unwrap();
    wait_for_event(&mut runtime, |event| {
        event.operation_id == scan && event.phase == Phase::Started
    });
    backend.fault_next("stop_scan", Fault::Hang);
    backend.fault_next("disconnect", Fault::Hang);
    let started = Instant::now();
    backend.end_adapter_stream();
    wait_for_event(&mut runtime, |event| {
        event.operation == Operation::Initialize && event.phase == Phase::Failed
    });
    assert!(started.elapsed() < Duration::from_secs(1));
    success(&mut runtime, client.initialize());
    success(&mut runtime, client.connect(ADDRESS));
}

#[test]
fn a_closed_notification_stream_is_observable_and_can_be_restarted() {
    let (backend, mut runtime, client) = runtime();
    success(&mut runtime, client.initialize());
    success(&mut runtime, client.connect(ADDRESS));
    success(
        &mut runtime,
        client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC),
    );
    wait_pumps(&backend, 1);
    backend.end_notification_stream();
    let ended = wait_for_event(&mut runtime, |event| {
        event.operation == Operation::Notification
    });
    assert_eq!(ended.phase, Phase::Failed);
    assert!(!ended.terminal);
    success(
        &mut runtime,
        client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC),
    );
    wait_pumps(&backend, 2);
    backend.emit_notification(CHARACTERISTIC, vec![9]);
    let value = wait_for_event(&mut runtime, |event| {
        event.operation == Operation::Notification
    });
    assert_eq!(value.data, EventData::Bytes(vec![9]));
}

pub(super) fn state() -> (CoreState, mpsc::Receiver<BleEvent>) {
    let (event_tx, event_rx) = mpsc::channel(32);
    let (internal_tx, _) = mpsc::channel(32);
    let outbox = Arc::new(EventOutbox::new(event_tx));
    outbox.reserve_request(OperationId::new(99)).unwrap();
    outbox.reserve_signal(SignalKey::Adapter).unwrap();
    outbox
        .reserve_signal(SignalKey::Device(DeviceId::parse(ADDRESS).unwrap()))
        .unwrap();
    outbox
        .reserve_signal(SignalKey::Notification(DeviceId::parse(ADDRESS).unwrap()))
        .unwrap();
    (
        CoreState {
            data: Arc::new(DataInbox::default()),
            adapter_tasks: JoinSet::new(),
            adapter_attached: true,
            adapter_state: AdapterState::PoweredOn,
            snapshots: HashMap::new(),
            work_generation: 0,
            backend: Arc::new(FakeBackend::default()),
            outbox,
            internal_tx,
            initialized: true,
            initializing: false,
            adapter_info: None,
            scan: ScanRuntimeState::default(),
            known_devices: HashMap::new(),
            devices: HashMap::new(),
            shutdown: CancellationToken::new(),
            scan_control: Arc::new(ScanControl::default()),
            options: CoreOptions::default(),
            adapter_generation: 7,
        },
        event_rx,
    )
}

#[tokio::test]
async fn old_notification_values_errors_and_endings_cannot_affect_a_new_pump() {
    let (mut state, mut events) = state();
    let device = DeviceId::parse(ADDRESS).unwrap();
    let key = GattKey::parse(SERVICE, CHARACTERISTIC).unwrap();
    state.devices.insert(
        device.clone(),
        DeviceSession {
            state: ConnectionState::Connected,
            subscriptions: HashSet::from([key]),
            notification_generation: 2,
            notification_routes: Arc::new(std::sync::Mutex::new(HashMap::from([(
                CHARACTERISTIC.into(),
                OperationId::new(99),
            )]))),
            notification_cancellation: Some(CancellationToken::new()),
            ..DeviceSession::default()
        },
    );
    let mut tasks = JoinSet::new();
    state
        .handle_internal(
            InternalEvent::Notification {
                device: device.clone(),
                generation: 1,
                route: Some(OperationId::new(99)),
                notification: BackendNotification {
                    characteristic_uuid: CHARACTERISTIC.to_string(),
                    value: vec![1],
                },
            },
            &mut tasks,
        )
        .await;
    state
        .handle_internal(
            InternalEvent::NotificationPumpFailed {
                device: device.clone(),
                generation: 1,
                error: BleError::OperationFailed("old error".to_string()),
            },
            &mut tasks,
        )
        .await;
    state
        .handle_internal(
            InternalEvent::NotificationPumpEnded {
                device: device.clone(),
                generation: 1,
            },
            &mut tasks,
        )
        .await;
    assert!(events.try_recv().is_err());
    assert!(state.devices[&device].notification_cancellation.is_some());
    state
        .handle_internal(
            InternalEvent::Notification {
                device: device.clone(),
                generation: 2,
                route: Some(OperationId::new(99)),
                notification: BackendNotification {
                    characteristic_uuid: CHARACTERISTIC.to_string(),
                    value: vec![2],
                },
            },
            &mut tasks,
        )
        .await;
    assert_eq!(events.try_recv().unwrap().data, EventData::Bytes(vec![2]));
    state
        .handle_internal(
            InternalEvent::NotificationPumpEnded {
                device: device.clone(),
                generation: 2,
            },
            &mut tasks,
        )
        .await;
    assert_eq!(events.try_recv().unwrap().phase, Phase::Failed);
    assert!(state.devices[&device].notification_cancellation.is_none());
}

#[tokio::test]
async fn stale_adapter_events_do_not_clear_a_reinitialized_adapter() {
    let (mut state, mut events) = state();
    state
        .handle_internal(
            InternalEvent::AdapterStreamEnded {
                generation: 6,
                error: BleError::OperationFailed("old stream".to_string()),
            },
            &mut JoinSet::new(),
        )
        .await;
    assert!(state.initialized);
    assert!(events.try_recv().is_err());
}

#[tokio::test]
async fn request_rejection_reports_the_existing_connection_state() {
    let (mut state, mut events) = state();
    let device = DeviceId::parse(ADDRESS).unwrap();
    let mut session = DeviceSession {
        state: ConnectionState::Connected,
        ..DeviceSession::default()
    };
    for index in 0..COMMAND_CHANNEL_CAPACITY {
        session.pending.push_back(QueuedOperation {
            operation_id: OperationId::new(index as i64 + 1),
            action: DeviceAction::Read(GattKey::parse(SERVICE, CHARACTERISTIC).unwrap()),
        });
    }
    state.devices.insert(device.clone(), session);
    state
        .device_command(
            OperationId::new(99),
            device,
            DeviceAction::Connect,
            &mut JoinSet::new(),
        )
        .await;
    let event = events.try_recv().unwrap();
    assert_eq!(event.connection_state, Some(ConnectionState::Connected));
    assert_eq!(event.error.unwrap().code, "QUEUE_FULL");
}
