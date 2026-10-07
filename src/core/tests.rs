use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::backend::test_support::FakeBackend;

use super::*;

pub(super) fn wait_for_event(
    runtime: &mut CoreRuntime,
    predicate: impl Fn(&BleEvent) -> bool,
) -> BleEvent {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        while let Some(event) = runtime.try_recv() {
            if predicate(&event) {
                return event;
            }
        }
        assert!(Instant::now() < deadline, "timed out waiting for BLE event");
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn device_id_normalizes_case_and_whitespace() {
    let id = DeviceId::parse("  AA:BB:CC:DD:EE:FF  ").expect("valid device id");
    assert_eq!(id.as_str(), "aa:bb:cc:dd:ee:ff");
}

#[test]
fn device_id_rejects_empty_input() {
    let error = DeviceId::parse("   ").expect_err("empty device id must fail");
    assert_eq!(error.code(), "INVALID_ARGUMENT");
}

#[test]
fn gatt_key_normalizes_uuid_strings() {
    let key = GattKey::parse(
        "0000180D-0000-1000-8000-00805F9B34FB",
        "00002A37-0000-1000-8000-00805F9B34FB",
    )
    .expect("valid GATT UUIDs");
    assert_eq!(key.service_uuid(), "0000180d-0000-1000-8000-00805f9b34fb");
}

#[test]
fn scan_session_rejects_a_second_start_while_active() {
    let mut session = ScanSession::default();
    session
        .start(OperationId::new(1), None)
        .expect("first scan starts");
    let error = session
        .start(OperationId::new(2), None)
        .expect_err("second scan must fail");
    assert_eq!(error.code(), "BUSY");
}

#[test]
fn stale_scan_generation_cannot_finish_the_current_scan() {
    let mut session = ScanSession::default();
    let first = session
        .start(OperationId::new(1), None)
        .expect("first scan starts");
    session.finish(first).expect("first scan finishes");
    let second = session
        .start(OperationId::new(2), None)
        .expect("second scan starts");
    assert!(!session.finish(first).expect("stale finish is ignored"));
    assert_eq!(session.active_generation(), Some(second));
}

#[test]
fn failed_event_is_terminal_and_exposes_retryability() {
    let event = BleEvent::failed(
        Operation::Scan,
        OperationId::new(9),
        EventContext::default(),
        BleError::Busy("scan already active".to_string()),
    );
    assert!(event.terminal);
    assert!(!event.error.expect("error payload").retryable);
}

#[test]
fn failed_scan_start_emits_terminal_event_and_allows_retry() {
    let backend = Arc::new(FakeBackend::default());
    backend.fail_next_scan_start(BleError::ScanFailed("radio unavailable".to_string()));
    let mut runtime = CoreRuntime::spawn_with_backend(backend);
    let client = runtime.client();
    let init_id = client.initialize().expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == init_id && event.phase == Phase::Succeeded
    });
    let failed_id = client.start_scan(None).expect("submission accepted");
    let failed = wait_for_event(&mut runtime, |event| {
        event.operation_id == failed_id && event.terminal
    });
    assert_eq!(failed.phase, Phase::Failed);
    let retry_id = client.start_scan(None).expect("submission accepted");
    let started = wait_for_event(&mut runtime, |event| {
        event.operation_id == retry_id && event.phase == Phase::Started
    });
    assert_eq!(started.operation_id, retry_id);
}

#[test]
fn duplicate_scan_is_rejected_without_replacing_active_scan() {
    let backend = Arc::new(FakeBackend::default());
    let mut runtime = CoreRuntime::spawn_with_backend(backend);
    let client = runtime.client();
    let init_id = client.initialize().expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == init_id && event.phase == Phase::Succeeded
    });
    let active_id = client.start_scan(None).expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == active_id && event.phase == Phase::Started
    });
    let rejected_id = client.start_scan(None).expect("submission accepted");
    let rejected = wait_for_event(&mut runtime, |event| {
        event.operation_id == rejected_id && event.terminal
    });
    assert_eq!(rejected.error.expect("busy error").code, "BUSY");
}

#[test]
fn timed_scan_emits_exactly_one_terminal_event() {
    let backend = Arc::new(FakeBackend::default());
    let mut runtime = CoreRuntime::spawn_with_backend(backend);
    let client = runtime.client();
    let init_id = client.initialize().expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == init_id && event.phase == Phase::Succeeded
    });
    let scan_id = client
        .start_scan(Some(Duration::from_millis(10)))
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == scan_id && event.phase == Phase::Started
    });
    let terminal = wait_for_event(&mut runtime, |event| {
        event.operation_id == scan_id && event.terminal
    });
    assert_eq!(terminal.phase, Phase::Succeeded);

    thread::sleep(Duration::from_millis(25));
    let mut duplicate_terminal = false;
    while let Some(event) = runtime.try_recv() {
        duplicate_terminal |= event.operation_id == scan_id && event.terminal;
    }
    assert!(!duplicate_terminal);
}

#[test]
fn remote_disconnect_uses_unsolicited_operation_id() {
    let backend = Arc::new(FakeBackend::default());
    backend.add_device(DeviceInfo::new(
        "AA:BB:CC:DD:EE:FF".to_string(),
        Some("Sensor".to_string()),
        Some(-42),
        Vec::new(),
        Default::default(),
        Default::default(),
        None,
    ));
    let mut runtime = CoreRuntime::spawn_with_backend(backend.clone());
    let client = runtime.client();
    let init_id = client.initialize().expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == init_id && event.phase == Phase::Succeeded
    });
    backend.emit_device_discovered("AA:BB:CC:DD:EE:FF");
    wait_for_event(&mut runtime, |event| {
        event.operation == Operation::Scan && event.phase == Phase::Progress
    });
    let connect_id = client
        .connect("aa:bb:cc:dd:ee:ff")
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == connect_id && event.phase == Phase::Succeeded
    });
    backend.emit_remote_disconnect("AA:BB:CC:DD:EE:FF");
    let disconnected = wait_for_event(&mut runtime, |event| {
        event.operation == Operation::Disconnect && event.phase == Phase::Received
    });
    assert_eq!(disconnected.operation_id, OperationId::UNSOLICITED);
}

#[test]
fn remote_disconnect_cancels_a_stale_connect_before_reconnect() {
    let backend = Arc::new(FakeBackend::default());
    backend.add_device(DeviceInfo::new(
        "AA:BB:CC:DD:EE:FF".to_string(),
        None,
        None,
        Vec::new(),
        Default::default(),
        Default::default(),
        None,
    ));
    let mut runtime = CoreRuntime::spawn_with_backend(backend.clone());
    let client = runtime.client();
    let init_id = client.initialize().expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == init_id && event.phase == Phase::Succeeded
    });

    backend.block_next_connect();
    let stale_id = client
        .connect("AA:BB:CC:DD:EE:FF")
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == stale_id && event.phase == Phase::Started
    });
    backend.emit_remote_disconnect("AA:BB:CC:DD:EE:FF");
    let cancelled = wait_for_event(&mut runtime, |event| {
        event.operation_id == stale_id && event.terminal
    });
    assert_eq!(cancelled.phase, Phase::Cancelled);

    let reconnect_id = client
        .connect("AA:BB:CC:DD:EE:FF")
        .expect("submission accepted");
    let reconnected = wait_for_event(&mut runtime, |event| {
        event.operation_id == reconnect_id && event.terminal
    });
    assert_eq!(reconnected.phase, Phase::Succeeded);
}

#[test]
fn failed_connect_returns_to_disconnected_and_can_retry() {
    let backend = Arc::new(FakeBackend::default());
    backend.add_device(DeviceInfo::new(
        "AA:BB:CC:DD:EE:FF".to_string(),
        None,
        None,
        Vec::new(),
        Default::default(),
        Default::default(),
        None,
    ));
    backend.fail_next_connect(BleError::ConnectionFailed("rejected".to_string()));
    let mut runtime = CoreRuntime::spawn_with_backend(backend);
    let client = runtime.client();
    let init_id = client.initialize().expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == init_id && event.phase == Phase::Succeeded
    });

    let failed_id = client
        .connect("AA:BB:CC:DD:EE:FF")
        .expect("submission accepted");
    let failed = wait_for_event(&mut runtime, |event| {
        event.operation_id == failed_id && event.terminal
    });
    assert_eq!(failed.phase, Phase::Failed);

    let retry_id = client
        .connect("AA:BB:CC:DD:EE:FF")
        .expect("submission accepted");
    let retried = wait_for_event(&mut runtime, |event| {
        event.operation_id == retry_id && event.terminal
    });
    assert_eq!(retried.phase, Phase::Succeeded);
}

#[test]
fn duplicate_characteristic_uuid_across_services_is_rejected() {
    let backend = Arc::new(FakeBackend::default());
    backend.add_device(DeviceInfo::new(
        "AA:BB:CC:DD:EE:FF".to_string(),
        None,
        None,
        Vec::new(),
        Default::default(),
        Default::default(),
        None,
    ));
    let mut runtime = CoreRuntime::spawn_with_backend(backend);
    let client = runtime.client();
    let init_id = client.initialize().expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == init_id && event.phase == Phase::Succeeded
    });
    let connect_id = client
        .connect("AA:BB:CC:DD:EE:FF")
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == connect_id && event.phase == Phase::Succeeded
    });
    let first_id = client
        .subscribe(
            "AA:BB:CC:DD:EE:FF",
            "0000180d-0000-1000-8000-00805f9b34fb",
            "00002a37-0000-1000-8000-00805f9b34fb",
        )
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == first_id && event.phase == Phase::Succeeded
    });
    let second_id = client
        .subscribe(
            "AA:BB:CC:DD:EE:FF",
            "0000180f-0000-1000-8000-00805f9b34fb",
            "00002a37-0000-1000-8000-00805f9b34fb",
        )
        .expect("submission accepted");
    let rejected = wait_for_event(&mut runtime, |event| {
        event.operation_id == second_id && event.terminal
    });
    assert_eq!(
        rejected.error.expect("ambiguity error").code,
        "AMBIGUOUS_CHARACTERISTIC"
    );
}

#[test]
fn device_commands_are_executed_in_submission_order() {
    let backend = Arc::new(FakeBackend::default());
    backend.add_device(DeviceInfo::new(
        "AA:BB:CC:DD:EE:FF".to_string(),
        None,
        None,
        Vec::new(),
        Default::default(),
        Default::default(),
        None,
    ));
    let mut runtime = CoreRuntime::spawn_with_backend(backend);
    let client = runtime.client();
    let init_id = client.initialize().expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == init_id && event.phase == Phase::Succeeded
    });

    let connect_id = client
        .connect("AA:BB:CC:DD:EE:FF")
        .expect("submission accepted");
    let discover_id = client
        .discover_services("AA:BB:CC:DD:EE:FF")
        .expect("submission accepted");
    let discovered = wait_for_event(&mut runtime, |event| {
        event.operation_id == discover_id && event.terminal
    });

    assert_eq!(discovered.phase, Phase::Succeeded);
    assert!(connect_id.get() < discover_id.get());
}

#[test]
fn write_response_mode_is_preserved_through_the_command_seam() {
    let backend = Arc::new(FakeBackend::default());
    backend.add_device(DeviceInfo::new(
        "AA:BB:CC:DD:EE:FF".to_string(),
        None,
        None,
        Vec::new(),
        Default::default(),
        Default::default(),
        None,
    ));
    let mut runtime = CoreRuntime::spawn_with_backend(backend.clone());
    let client = runtime.client();
    let init_id = client.initialize().expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == init_id && event.phase == Phase::Succeeded
    });
    let connect_id = client
        .connect("AA:BB:CC:DD:EE:FF")
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == connect_id && event.phase == Phase::Succeeded
    });

    let service = "0000180d-0000-1000-8000-00805f9b34fb";
    let characteristic = "00002a37-0000-1000-8000-00805f9b34fb";
    let with_response = client
        .write("AA:BB:CC:DD:EE:FF", service, characteristic, vec![1], true)
        .expect("submission accepted");
    let without_response = client
        .write("AA:BB:CC:DD:EE:FF", service, characteristic, vec![2], false)
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == with_response && event.phase == Phase::Succeeded
    });
    wait_for_event(&mut runtime, |event| {
        event.operation_id == without_response && event.phase == Phase::Succeeded
    });
    assert_eq!(backend.write_modes(), vec![true, false]);
}

#[test]
fn notification_routing_uses_one_pump_and_retains_failed_unsubscribe() {
    let backend = Arc::new(FakeBackend::default());
    backend.add_connected_device("AA:BB:CC:DD:EE:FF");
    let mut runtime = CoreRuntime::spawn_with_backend(backend.clone());
    let client = runtime.client();
    let init_id = client.initialize().expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == init_id && event.phase == Phase::Succeeded
    });
    let connect_id = client
        .connect("AA:BB:CC:DD:EE:FF")
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == connect_id && event.phase == Phase::Succeeded
    });

    let service_a = "0000180d-0000-1000-8000-00805f9b34fb";
    let service_b = "0000180f-0000-1000-8000-00805f9b34fb";
    let characteristic = "00002a37-0000-1000-8000-00805f9b34fb";
    let subscribe_id = client
        .subscribe("AA:BB:CC:DD:EE:FF", service_a, characteristic)
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == subscribe_id && event.phase == Phase::Succeeded
    });
    let duplicate_id = client
        .subscribe("AA:BB:CC:DD:EE:FF", service_a, characteristic)
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == duplicate_id && event.phase == Phase::Succeeded
    });
    assert_eq!(backend.notification_pumps_started(), 1);

    backend.emit_notification(characteristic, vec![1, 2, 3]);
    let notification = wait_for_event(&mut runtime, |event| {
        event.operation == Operation::Notification && event.phase == Phase::Received
    });
    assert_eq!(notification.context.service_uuid, service_a);
    assert_eq!(notification.context.characteristic_uuid, characteristic);
    assert_eq!(notification.operation_id, OperationId::UNSOLICITED);

    backend.emit_notification_error(BleError::SubscribeFailed(
        "notification stream ended".to_string(),
    ));
    wait_for_event(&mut runtime, |event| {
        event.operation == Operation::Notification && event.phase == Phase::Failed
    });
    let restart_id = client
        .subscribe("AA:BB:CC:DD:EE:FF", service_a, characteristic)
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == restart_id && event.phase == Phase::Succeeded
    });
    let restart_deadline = Instant::now() + Duration::from_secs(1);
    while backend.notification_pumps_started() != 2 {
        assert!(Instant::now() < restart_deadline);
        thread::sleep(Duration::from_millis(5));
    }

    backend.fail_next_unsubscribe(BleError::UnsubscribeFailed("radio busy".to_string()));
    let unsubscribe_id = client
        .unsubscribe("AA:BB:CC:DD:EE:FF", service_a, characteristic)
        .expect("submission accepted");
    let unsubscribe = wait_for_event(&mut runtime, |event| {
        event.operation_id == unsubscribe_id && event.terminal
    });
    assert_eq!(unsubscribe.phase, Phase::Failed);

    let ambiguous_id = client
        .subscribe("AA:BB:CC:DD:EE:FF", service_b, characteristic)
        .expect("submission accepted");
    let ambiguous = wait_for_event(&mut runtime, |event| {
        event.operation_id == ambiguous_id && event.terminal
    });
    assert_eq!(
        ambiguous.error.expect("ambiguity error").code,
        "AMBIGUOUS_CHARACTERISTIC"
    );
    assert_eq!(backend.notification_pumps_started(), 2);

    backend.emit_remote_disconnect("AA:BB:CC:DD:EE:FF");
    wait_for_event(&mut runtime, |event| {
        event.operation == Operation::Disconnect && event.phase == Phase::Received
    });
    let reconnect_id = client
        .connect("AA:BB:CC:DD:EE:FF")
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == reconnect_id && event.phase == Phase::Succeeded
    });
    let after_disconnect_id = client
        .subscribe("AA:BB:CC:DD:EE:FF", service_b, characteristic)
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == after_disconnect_id && event.phase == Phase::Succeeded
    });
}

#[test]
fn shutdown_completes_after_many_local_rejections() {
    let backend = Arc::new(FakeBackend::default());
    let mut runtime = CoreRuntime::spawn_with_backend(backend);
    let client = runtime.client();
    for _ in 0..(EVENT_CHANNEL_CAPACITY * 2) {
        let event = client
            .connect("   ")
            .expect_err("invalid address rejected locally");
        assert!(event.terminal);
    }

    let started = Instant::now();
    runtime.shutdown();
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn saturated_command_submission_returns_queue_full() {
    let backend = Arc::new(FakeBackend::default());
    backend.add_device(DeviceInfo::new(
        "AA:BB:CC:DD:EE:FF".to_string(),
        None,
        None,
        Vec::new(),
        Default::default(),
        Default::default(),
        None,
    ));
    let mut runtime = CoreRuntime::spawn_with_backend(backend.clone());
    let client = runtime.client();
    let init_id = client.initialize().expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == init_id && event.phase == Phase::Succeeded
    });

    backend.block_next_connect();
    let connect_id = client
        .connect("AA:BB:CC:DD:EE:FF")
        .expect("submission accepted");
    wait_for_event(&mut runtime, |event| {
        event.operation_id == connect_id && event.phase == Phase::Started
    });
    for _ in 0..(COMMAND_CHANNEL_CAPACITY * 2) {
        if let Err(event) = client.discover_services("AA:BB:CC:DD:EE:FF") {
            assert_eq!(event.error.as_ref().unwrap().code, "QUEUE_FULL");
        }
    }
    let queue_full = wait_for_event(&mut runtime, |event| {
        event
            .error
            .as_ref()
            .is_some_and(|error| error.code == "QUEUE_FULL")
    });
    assert!(queue_full.terminal);
    backend.release_connect();
}
