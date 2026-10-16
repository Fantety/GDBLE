//! Regression cases promoted from the architecture review and its race fixtures.
use super::regression_tests::{runtime, state, success};
use super::tests::wait_for_event;
use super::*;
use crate::backend::test_support::Fault;
use std::thread;
use std::time::Instant;

const ADDRESS: &str = "aa:bb:cc:dd:ee:ff";
const SERVICE: &str = "0000180d-0000-1000-8000-00805f9b34fb";
const CHARACTERISTIC: &str = "00002a37-0000-1000-8000-00805f9b34fb";
const OTHER: &str = "00002a38-0000-1000-8000-00805f9b34fb";

#[test]
fn local_disconnect_echo_before_result_preserves_queued_reconnect() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    success(&mut core, client.connect(ADDRESS));
    backend.echo_local_disconnect();
    backend.fault_next("disconnect_after_event", Fault::Hang);
    let disconnect = client.disconnect(ADDRESS).unwrap();
    let reconnect = client.connect(ADDRESS).unwrap();
    let ended = wait_for_event(&mut core, |event| {
        event.operation_id == disconnect && event.terminal
    });
    assert_eq!(
        ended.phase,
        Phase::Succeeded,
        "physical confirmation wins over timeout"
    );
    let connected = wait_for_event(&mut core, |event| {
        event.operation_id == reconnect && event.terminal
    });
    assert_eq!(connected.phase, Phase::Succeeded);
    backend.emit_disconnect_echo(ADDRESS);
    backend.emit_disconnect_echo(ADDRESS);
    success(&mut core, client.read(ADDRESS, SERVICE, CHARACTERISTIC));
    assert!(backend.is_connected(ADDRESS));
    while let Some(event) = core.try_recv() {
        assert!(
            !event.terminal
                || (event.operation_id != disconnect && event.operation_id != reconnect)
        );
    }
}

#[test]
fn late_echo_during_reconnect_is_verified_but_real_disconnect_still_invalidates() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    success(&mut core, client.connect(ADDRESS));
    success(&mut core, client.disconnect(ADDRESS));
    backend.emit_disconnect_echo(ADDRESS); // Duplicate in disconnected state.
    backend.block_next_connect();
    let id = client.connect(ADDRESS).unwrap();
    wait_for_event(&mut core, |e| {
        e.operation_id == id && e.phase == Phase::Started
    });
    backend.emit_disconnect_echo(ADDRESS);
    thread::sleep(Duration::from_millis(10));
    backend.release_connect();
    assert_eq!(
        wait_for_event(&mut core, |e| e.operation_id == id && e.terminal).phase,
        Phase::Succeeded
    );
    backend.emit_remote_disconnect(ADDRESS);
    let event = wait_for_event(&mut core, |e| {
        e.operation == Operation::Disconnect && e.phase == Phase::Received
    });
    assert_eq!(event.connection_state, Some(ConnectionState::Disconnected));
    backend.emit_disconnect_echo(ADDRESS);
    thread::sleep(Duration::from_millis(10));
    assert!(core.try_recv().is_none());
}

#[test]
fn failed_connection_verification_invalidates_and_cleans_up() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    success(&mut core, client.connect(ADDRESS));
    success(&mut core, client.disconnect(ADDRESS));
    success(&mut core, client.connect(ADDRESS));
    backend.fault_next("is_connected", Fault::Hang);
    backend.emit_disconnect_echo(ADDRESS);
    let event = wait_for_event(&mut core, |e| {
        e.operation == Operation::Disconnect && e.error.is_some()
    });
    assert_eq!(event.error.unwrap().code, "TIMEOUT");
    assert!(!event.terminal);
    let event = wait_for_event(&mut core, |e| {
        e.operation == Operation::Disconnect && e.phase == Phase::Received
    });
    assert_eq!(event.connection_state, Some(ConnectionState::Disconnected));
    success(&mut core, client.connect(ADDRESS));
}

#[test]
fn first_and_additional_subscription_deliver_immediate_values_after_success() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    success(&mut core, client.connect(ADDRESS));
    for (index, characteristic) in [CHARACTERISTIC, OTHER].iter().enumerate() {
        backend.notify_on_subscribe(vec![index as u8]);
        let id = client.subscribe(ADDRESS, SERVICE, characteristic).unwrap();
        let mut succeeded = false;
        loop {
            let event = wait_for_event(&mut core, |_| true);
            if event.operation_id == id && event.terminal {
                assert_eq!(event.phase, Phase::Succeeded);
                succeeded = true;
            }
            if event.operation == Operation::Notification {
                assert!(succeeded, "notification overtook subscribe completion");
                assert_eq!(event.context.service_uuid, SERVICE);
                assert_eq!(event.context.characteristic_uuid, *characteristic);
                assert_eq!(event.data, EventData::Bytes(vec![index as u8]));
                break;
            }
        }
    }
    assert_eq!(backend.notification_pumps_started(), 1);
}

#[test]
fn failed_subscription_discards_its_values_and_retry_really_subscribes() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    success(&mut core, client.connect(ADDRESS));
    success(&mut core, client.subscribe(ADDRESS, SERVICE, OTHER));
    backend.notify_on_subscribe(vec![1]);
    backend.fail_subscribe(BleError::SubscribeFailed("injected failure".into()));
    let id = client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC).unwrap();
    let failed = wait_for_event(&mut core, |e| e.operation_id == id && e.terminal);
    assert_eq!(failed.phase, Phase::Failed);
    thread::sleep(Duration::from_millis(10));
    assert!(core.try_recv().is_none());
    backend.notify_on_subscribe(vec![2]);
    success(
        &mut core,
        client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC),
    );
    assert_eq!(
        wait_for_event(&mut core, |e| e.operation == Operation::Notification).data,
        EventData::Bytes(vec![2])
    );
    backend.emit_notification(OTHER, vec![3]);
    assert_eq!(
        wait_for_event(&mut core, |e| e.operation == Operation::Notification).data,
        EventData::Bytes(vec![3])
    );
}

#[test]
fn oversized_startup_notification_fails_and_resubscribe_rebuilds_pump() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    success(&mut core, client.connect(ADDRESS));
    backend.notify_on_subscribe(vec![0; 65537]);
    backend.fault_next("subscribe_after_notification", Fault::Hang);
    let id = client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC).unwrap();
    let event = wait_for_event(&mut core, |e| e.operation_id == id && e.terminal);
    assert_eq!(event.error.unwrap().code, "EVENT_OVERFLOW");
    let event = wait_for_event(&mut core, |e| e.operation == Operation::Notification);
    assert_eq!(event.error.unwrap().code, "EVENT_OVERFLOW");
    assert!(!event.terminal);
    assert!(backend.is_connected(ADDRESS));
    success(
        &mut core,
        client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC),
    );
    assert_eq!(backend.notification_pumps_started(), 2);
    backend.emit_notification(CHARACTERISTIC, vec![7]);
    assert_eq!(
        wait_for_event(&mut core, |e| e.operation == Operation::Notification).data,
        EventData::Bytes(vec![7])
    );
}

#[tokio::test]
async fn startup_count_limit_clears_queued_data_and_invalidates_old_routes() {
    let (mut core, mut events) = state();
    let device = DeviceId::parse(ADDRESS).unwrap();
    let key = GattKey::parse(SERVICE, CHARACTERISTIC).unwrap();
    let id = OperationId::new(99);
    let mut tasks = JoinSet::new();
    core.devices.insert(
        device.clone(),
        DeviceSession {
            state: ConnectionState::Connected,
            subscribing: Some(key.clone()),
            notification_routes: Arc::new(std::sync::Mutex::new(HashMap::from([(
                CHARACTERISTIC.into(),
                id,
            )]))),
            notification_generation: 1,
            notification_ready: true,
            notification_cancellation: Some(CancellationToken::new()),
            in_flight: Some(InFlightOperation {
                operation_id: id,
                generation: 1,
                action: DeviceAction::Subscribe(key),
                abort_handle: tasks.spawn(futures::future::pending()),
            }),
            ..DeviceSession::default()
        },
    );
    for _ in 0..65 {
        core.notification(
            device.clone(),
            1,
            Some(id),
            BackendNotification {
                characteristic_uuid: CHARACTERISTIC.into(),
                value: vec![0],
            },
            &mut tasks,
        )
        .await;
    }
    assert!(core.devices[&device].notification_buffer.is_empty());
    assert!(core.devices[&device].subscriptions.is_empty());
    assert_eq!(core.devices[&device].state, ConnectionState::Connected);
    assert_eq!(
        events.try_recv().unwrap().error.unwrap().code,
        "EVENT_OVERFLOW"
    );
    assert_eq!(
        events.try_recv().unwrap().operation,
        Operation::Notification
    );
    assert!(events.try_recv().is_err());
}

#[test]
fn power_cycle_preserves_listener_but_requires_explicit_initialize() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    success(&mut core, client.connect(ADDRESS));
    let scan = client.start_scan(None).unwrap();
    wait_for_event(&mut core, |e| {
        e.operation_id == scan && e.phase == Phase::Started
    });
    backend.set_adapter_state(AdapterState::PoweredOff);
    let event = wait_for_event(&mut core, |e| e.operation == Operation::Initialize);
    assert_eq!(event.adapter_initialized, Some(false));
    assert_eq!(event.error.unwrap().code, "ADAPTER_UNAVAILABLE");
    assert_eq!(
        wait_for_event(&mut core, |e| e.operation_id == scan && e.terminal).phase,
        Phase::Cancelled
    );
    backend.set_adapter_state(AdapterState::PoweredOn);
    let event = wait_for_event(&mut core, |e| {
        e.operation == Operation::Initialize && e.phase == Phase::Progress
    });
    assert_eq!(event.adapter_initialized, Some(false));
    let id = client.connect(ADDRESS).unwrap();
    assert_eq!(
        wait_for_event(&mut core, |e| e.operation_id == id && e.terminal)
            .error
            .unwrap()
            .code,
        "NOT_INITIALIZED"
    );
    success(&mut core, client.initialize());
    success(&mut core, client.connect(ADDRESS));
}

#[test]
fn initially_unavailable_adapter_is_not_reported_initialized() {
    let (backend, mut core, client) = runtime();
    backend.set_adapter_state(AdapterState::PoweredOff);
    let id = client.initialize().unwrap();
    let event = wait_for_event(&mut core, |e| e.operation_id == id && e.terminal);
    assert_eq!(event.adapter_initialized, Some(false));
    assert_eq!(event.error.unwrap().code, "ADAPTER_UNAVAILABLE");
    backend.set_adapter_state(AdapterState::PoweredOn);
    wait_for_event(&mut core, |e| {
        e.operation == Operation::Initialize && e.phase == Phase::Progress
    });
    success(&mut core, client.initialize());
}

#[test]
fn advertisement_categories_work_without_device_updated_and_snapshots_have_deadlines() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    let scan = client.start_scan(None).unwrap();
    wait_for_event(&mut core, |e| {
        e.operation_id == scan && e.phase == Phase::Started
    });
    for update in [
        AdvertisementUpdate::Rssi(-42),
        AdvertisementUpdate::ManufacturerData(HashMap::from([(7, vec![1])])),
        AdvertisementUpdate::ServiceData(HashMap::from([(SERVICE.into(), vec![2])])),
        AdvertisementUpdate::Services(vec![SERVICE.into()]),
    ] {
        backend.emit_backend(BackendEvent::Changed {
            device: DeviceId::parse(ADDRESS).unwrap(),
            update: update.clone(),
        });
        wait_for_event(&mut core, |e| {
            if let EventData::Device { info, .. } = &e.data {
                let mut expected = info.clone();
                update.apply(&mut expected);
                info == &expected
            } else {
                false
            }
        });
    }
    backend.fault_next("snapshot", Fault::Hang);
    // Wait for the previous snapshot before beginning a new refresh.
    thread::sleep(Duration::from_millis(10));
    backend.emit_backend(BackendEvent::Changed {
        device: DeviceId::parse(ADDRESS).unwrap(),
        update: AdvertisementUpdate::Refresh,
    });
    let event = wait_for_event(&mut core, |e| {
        e.error.as_ref().is_some_and(|e| e.code == "TIMEOUT")
    });
    assert!(!event.terminal);
    backend.fault_next("snapshot", Fault::Hang);
    backend.emit_backend(BackendEvent::Changed {
        device: DeviceId::parse(ADDRESS).unwrap(),
        update: AdvertisementUpdate::Refresh,
    });
    backend.set_adapter_state(AdapterState::Unknown);
    let start = Instant::now();
    wait_for_event(&mut core, |e| {
        e.operation == Operation::Initialize && e.error.is_some()
    });
    assert!(
        start.elapsed() < Duration::from_millis(90),
        "snapshot blocked adapter event consumption"
    );
}

#[test]
fn services_modified_invalidates_gatt_and_requires_rediscovery() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    success(&mut core, client.connect(ADDRESS));
    success(
        &mut core,
        client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC),
    );
    backend.emit_backend(BackendEvent::ServicesChanged(
        DeviceId::parse(ADDRESS).unwrap(),
    ));
    let event = wait_for_event(&mut core, |e| e.operation == Operation::DiscoverServices);
    assert_eq!(event.error.unwrap().code, "SERVICES_CHANGED");
    assert!(!event.terminal);
    let id = client.read(ADDRESS, SERVICE, CHARACTERISTIC).unwrap();
    assert_eq!(
        wait_for_event(&mut core, |e| e.operation_id == id && e.terminal)
            .error
            .unwrap()
            .code,
        "SERVICES_CHANGED"
    );
    success(&mut core, client.discover_services(ADDRESS));
    success(
        &mut core,
        client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC),
    );
    assert_eq!(backend.notification_pumps_started(), 2);
    assert!(backend.is_connected(ADDRESS));
}

#[test]
fn stopped_consumer_does_not_block_scan_stop_or_accepted_disconnect() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    success(&mut core, client.connect(ADDRESS));
    let scan = client.start_scan(None).unwrap();
    wait_for_event(&mut core, |e| {
        e.operation_id == scan && e.phase == Phase::Started
    });
    backend.fault_next("read", Fault::Hang);
    let read = client.read(ADDRESS, SERVICE, CHARACTERISTIC).unwrap();
    let disconnect = client.disconnect(ADDRESS).unwrap();
    let mut expected = HashSet::from([read, disconnect, scan]);
    for _ in 0..4000 {
        if let Ok(id) = client.initialize() {
            expected.insert(id);
        }
    }
    client.stop_scan(scan);
    let deadline = Instant::now() + Duration::from_secs(1);
    while backend.stop_scan_calls() == 0 || backend.is_connected(ADDRESS) {
        assert!(
            Instant::now() < deadline,
            "control loop blocked on outbound events"
        );
        thread::sleep(Duration::from_millis(1));
    }
    let mut completed = HashSet::new();
    while completed.len() < expected.len() {
        let event = wait_for_event(&mut core, |_| true);
        if event.terminal {
            assert!(completed.insert(event.operation_id));
        }
    }
    assert_eq!(expected, completed);
    success(&mut core, client.initialize());
}

#[test]
fn partial_request_reservations_and_retired_notices_release_capacity() {
    let (sender, mut receiver) = mpsc::channel(3);
    let outbox = EventOutbox::new(sender.clone());
    let id = OperationId::new(1);
    outbox.reserve_request(id).unwrap();
    assert!(outbox.reserve_request(OperationId::new(2)).is_err());
    assert_eq!(sender.capacity(), 1);
    outbox.release_request(id);
    assert_eq!(sender.capacity(), 3);
    for _ in 0..2048 {
        outbox.reserve_signal(SignalKey::Adapter).unwrap();
        outbox.notice(
            &SignalKey::Adapter,
            BleEvent::received(
                Operation::Initialize,
                EventContext::default(),
                EventData::None,
            ),
        );
        outbox.retire_signal(&SignalKey::Adapter);
        assert!(receiver.try_recv().is_ok());
        outbox.flush();
        assert_eq!(sender.capacity(), 3);
    }
}

#[test]
fn outbound_overflow_stops_only_notifications_and_rebuilds_on_retry() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    success(&mut core, client.connect(ADDRESS));
    success(
        &mut core,
        client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC),
    );
    let mut accepted = HashSet::new();
    // Stop consuming until the event queue and reservations exhaust admission.
    let deadline = Instant::now() + Duration::from_millis(100);
    while Instant::now() < deadline {
        for _ in 0..64 {
            if let Ok(id) = client.initialize() {
                accepted.insert(id);
            }
        }
        thread::sleep(Duration::from_millis(1));
    }
    assert!(accepted.len() > 1000);
    for _ in 0..4 {
        backend.emit_notification(CHARACTERISTIC, vec![8]);
    }
    thread::sleep(Duration::from_millis(20));
    let mut failed = false;
    while !failed || !accepted.is_empty() {
        let event = wait_for_event(&mut core, |_| true);
        if event.terminal {
            assert!(accepted.remove(&event.operation_id));
        }
        if event.operation == Operation::Notification && event.phase == Phase::Failed {
            assert!(!failed);
            assert_eq!(event.error.unwrap().code, "EVENT_OVERFLOW");
            failed = true;
        }
    }
    assert!(backend.is_connected(ADDRESS));
    success(
        &mut core,
        client.subscribe(ADDRESS, SERVICE, CHARACTERISTIC),
    );
    assert_eq!(backend.notification_pumps_started(), 2);
    backend.emit_notification(CHARACTERISTIC, vec![9]);
    assert_eq!(
        wait_for_event(&mut core, |e| e.operation == Operation::Notification).data,
        EventData::Bytes(vec![9])
    );
}

#[tokio::test]
async fn data_inbox_coalesces_scan_and_never_evicts_notifications_silently() {
    let inbox = DataInbox::default();
    let changed = |index: usize, rssi| InternalEvent::Backend {
        generation: 1,
        result: Ok(BackendEvent::Changed {
            device: DeviceId::parse(&format!("device-{index}")).unwrap(),
            update: AdvertisementUpdate::Rssi(rssi),
        }),
    };
    assert!(!inbox.push(changed(0, -1)).ok().unwrap());
    assert!(!inbox.push(changed(0, -2)).ok().unwrap());
    let event = inbox.recv().await;
    assert!(matches!(
        event,
        InternalEvent::Backend {
            result: Ok(BackendEvent::Changed {
                update: AdvertisementUpdate::Rssi(-2),
                ..
            }),
            ..
        }
    ));
    for index in 0..INTERNAL_CHANNEL_CAPACITY {
        assert!(!inbox.push(changed(index, -1)).ok().unwrap());
    }
    assert!(inbox
        .push(changed(INTERNAL_CHANNEL_CAPACITY, -1))
        .ok()
        .unwrap());
    assert!(
        matches!(inbox.recv().await, InternalEvent::Backend { result: Ok(BackendEvent::Changed { device, .. }), .. } if device.as_str() == "device-1")
    );
    let inbox = DataInbox::default();
    for _ in 0..INTERNAL_CHANNEL_CAPACITY {
        assert!(inbox
            .push(InternalEvent::Notification {
                device: DeviceId::parse(ADDRESS).unwrap(),
                generation: 1,
                route: Some(OperationId::new(1)),
                notification: BackendNotification {
                    characteristic_uuid: CHARACTERISTIC.into(),
                    value: vec![1]
                }
            })
            .is_ok());
    }
    assert!(inbox.push(changed(0, -1)).ok().unwrap());
    assert!(matches!(
        inbox.recv().await,
        InternalEvent::Notification { .. }
    ));
}

#[test]
fn adapter_loss_rejects_discovery_data_already_queued_toward_godot() {
    let (sender, mut receiver) = mpsc::channel(1);
    let outbox = EventOutbox::new(sender);
    let mut old = BleEvent::progress(
        OperationId::new(1),
        EventContext::default(),
        EventData::None,
    );
    old.discovery_generation = Some(0);
    outbox.scan(old.clone());
    outbox.scan(old);
    outbox.discovery_epoch(1);
    assert!(!outbox.is_current(&receiver.try_recv().unwrap()));
    outbox.flush();
    assert!(
        receiver.try_recv().is_err(),
        "pending stale discovery was not cleared"
    );
}

#[test]
fn snapshot_failure_is_observable_without_an_active_scan() {
    let (backend, mut core, client) = runtime();
    success(&mut core, client.initialize());
    backend.fault_next("snapshot", Fault::Hang);
    backend.emit_backend(BackendEvent::Changed {
        device: DeviceId::parse(ADDRESS).unwrap(),
        update: AdvertisementUpdate::Refresh,
    });
    let event = wait_for_event(&mut core, |e| {
        e.operation == Operation::Scan && e.error.is_some()
    });
    assert_eq!(event.operation_id, OperationId::UNSOLICITED);
    assert!(!event.terminal);
    assert_eq!(event.error.unwrap().code, "TIMEOUT");
}
