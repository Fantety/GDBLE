use super::*;
use std::sync::Mutex;
use tokio::sync::Notify;

/// Rejections are delivered by the caller on the main thread, never queued.
pub type Submission = Result<OperationId, Box<BleEvent>>;

#[derive(Default)]
pub(super) struct ScanControl {
    requests: Mutex<HashMap<OperationId, CancellationToken>>,
    changed: Notify,
}

impl ScanControl {
    fn register(&self, operation_id: OperationId, token: CancellationToken) {
        self.requests.lock().unwrap().insert(operation_id, token);
    }

    pub(super) fn finish(&self, operation_id: OperationId) {
        self.requests.lock().unwrap().remove(&operation_id);
    }

    pub(super) async fn changed(&self) {
        self.changed.notified().await;
    }
}

#[derive(Clone)]
pub struct CoreClient {
    command_tx: mpsc::Sender<CoreCommand>,
    scan_control: Arc<ScanControl>,
    next_operation_id: Arc<AtomicI64>,
}

impl CoreClient {
    pub fn initialize(&self) -> Submission {
        let operation_id = self.next_id();
        self.send(
            Operation::Initialize,
            operation_id,
            EventContext::default(),
            CoreCommand::Initialize { operation_id },
        )
    }

    pub fn start_scan(&self, timeout: Option<Duration>) -> Submission {
        let operation_id = self.next_id();
        let cancellation = CancellationToken::new();
        self.scan_control
            .register(operation_id, cancellation.clone());
        let result = self.send(
            Operation::Scan,
            operation_id,
            EventContext::default(),
            CoreCommand::StartScan {
                operation_id,
                timeout,
                cancellation,
            },
        );
        if result.is_err() {
            self.scan_control.finish(operation_id);
        }
        result
    }

    pub fn stop_scan(&self, operation_id: OperationId) -> OperationId {
        if let Some(token) = self
            .scan_control
            .requests
            .lock()
            .unwrap()
            .get(&operation_id)
        {
            token.cancel();
            self.scan_control.changed.notify_one();
        }
        operation_id
    }

    pub fn connect(&self, address: &str) -> Submission {
        self.device_command(address, DeviceAction::Connect)
    }

    pub fn disconnect(&self, address: &str) -> Submission {
        self.device_command(address, DeviceAction::Disconnect)
    }

    pub fn discover_services(&self, address: &str) -> Submission {
        self.device_command(address, DeviceAction::DiscoverServices)
    }

    pub fn read(&self, address: &str, service_uuid: &str, characteristic_uuid: &str) -> Submission {
        self.gatt_command(
            Operation::Read,
            address,
            service_uuid,
            characteristic_uuid,
            DeviceAction::Read,
        )
    }

    pub fn write(
        &self,
        address: &str,
        service_uuid: &str,
        characteristic_uuid: &str,
        data: Vec<u8>,
        with_response: bool,
    ) -> Submission {
        self.gatt_command(
            Operation::Write,
            address,
            service_uuid,
            characteristic_uuid,
            |key| DeviceAction::Write {
                key,
                data,
                with_response,
            },
        )
    }

    pub fn subscribe(
        &self,
        address: &str,
        service_uuid: &str,
        characteristic_uuid: &str,
    ) -> Submission {
        self.gatt_command(
            Operation::Subscribe,
            address,
            service_uuid,
            characteristic_uuid,
            DeviceAction::Subscribe,
        )
    }

    pub fn unsubscribe(
        &self,
        address: &str,
        service_uuid: &str,
        characteristic_uuid: &str,
    ) -> Submission {
        self.gatt_command(
            Operation::Unsubscribe,
            address,
            service_uuid,
            characteristic_uuid,
            DeviceAction::Unsubscribe,
        )
    }

    pub fn reject(
        &self,
        operation: Operation,
        context: EventContext,
        error: BleError,
    ) -> Submission {
        Err(Box::new(BleEvent::failed(
            operation,
            self.next_id(),
            context,
            error,
        )))
    }

    fn next_id(&self) -> OperationId {
        OperationId::new(
            self.next_operation_id
                .fetch_add(1, Ordering::Relaxed)
                .max(1),
        )
    }

    fn send(
        &self,
        operation: Operation,
        operation_id: OperationId,
        context: EventContext,
        command: CoreCommand,
    ) -> Submission {
        self.command_tx
            .try_send(command)
            .map(|()| operation_id)
            .map_err(|error| {
                let error = match error {
                    mpsc::error::TrySendError::Full(_) => BleError::QueueFull,
                    mpsc::error::TrySendError::Closed(_) => {
                        BleError::Cancelled("BLE core is closed".to_string())
                    }
                };
                Box::new(BleEvent::failed(operation, operation_id, context, error))
            })
    }

    fn device_command(&self, address: &str, action: DeviceAction) -> Submission {
        let operation = action.operation();
        let operation_id = self.next_id();
        let device = DeviceId::parse(address).map_err(|error| {
            Box::new(BleEvent::failed(
                operation,
                operation_id,
                EventContext::default(),
                error,
            ))
        })?;
        let context = action.context(&device);
        self.send(
            operation,
            operation_id,
            context,
            CoreCommand::Device {
                operation_id,
                device,
                action,
            },
        )
    }

    fn gatt_command(
        &self,
        operation: Operation,
        address: &str,
        service_uuid: &str,
        characteristic_uuid: &str,
        action: impl FnOnce(GattKey) -> DeviceAction,
    ) -> Submission {
        let operation_id = self.next_id();
        let parsed = DeviceId::parse(address).and_then(|device| {
            GattKey::parse(service_uuid, characteristic_uuid).map(|key| (device, key))
        });
        let (device, key) = parsed.map_err(|error| {
            Box::new(BleEvent::failed(
                operation,
                operation_id,
                EventContext {
                    device_address: address.trim().to_ascii_lowercase(),
                    service_uuid: service_uuid.to_string(),
                    characteristic_uuid: characteristic_uuid.to_string(),
                },
                error,
            ))
        })?;
        let context = EventContext::for_gatt(&device, &key);
        self.send(
            operation,
            operation_id,
            context,
            CoreCommand::Device {
                operation_id,
                device,
                action: action(key),
            },
        )
    }
}

pub struct CoreRuntime {
    client: CoreClient,
    event_rx: mpsc::Receiver<BleEvent>,
    shutdown: CancellationToken,
    worker: Option<JoinHandle<()>>,
}

impl CoreRuntime {
    pub fn spawn_production() -> Self {
        Self::spawn_with_backend(Arc::new(BtleplugBackend::default()))
    }

    pub fn spawn_with_backend<B>(backend: Arc<B>) -> Self
    where
        B: BleBackend + 'static,
    {
        Self::spawn_with_options(backend, CoreOptions::default())
    }

    pub(super) fn spawn_with_options<B: BleBackend + 'static>(
        backend: Arc<B>,
        options: CoreOptions,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);
        let (event_tx, event_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let scan_control = Arc::new(ScanControl::default());
        let client = CoreClient {
            command_tx,
            scan_control: scan_control.clone(),
            next_operation_id: Arc::new(AtomicI64::new(1)),
        };
        let backend: Arc<dyn BleBackend> = backend;
        let shutdown = CancellationToken::new();
        let worker_shutdown = shutdown.clone();
        let worker = std::thread::Builder::new()
            .name("gdble-core".to_string())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_multi_thread()
                    .worker_threads(2)
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        eprintln!("[GDBLE] Failed to create Tokio runtime: {error}");
                        return;
                    }
                };
                let (done_tx, done_rx) = std::sync::mpsc::sync_channel(1);
                let core_task = runtime.spawn(async move {
                    run_core(
                        backend,
                        command_rx,
                        event_tx,
                        scan_control,
                        worker_shutdown,
                        options,
                    )
                    .await;
                    let _ = done_tx.send(());
                });
                let _ = done_rx.recv();
                drop(core_task);
                runtime.shutdown_timeout(Duration::from_millis(100));
            })
            .unwrap_or_else(|error| panic!("failed to spawn GDBLE core thread: {error}"));
        Self {
            client,
            event_rx,
            shutdown,
            worker: Some(worker),
        }
    }

    pub fn client(&self) -> CoreClient {
        self.client.clone()
    }

    pub fn try_recv(&mut self) -> Option<BleEvent> {
        self.event_rx.try_recv().ok()
    }

    pub fn shutdown(&mut self) {
        if let Some(worker) = self.worker.take() {
            self.shutdown.cancel();
            if worker.join().is_err() {
                eprintln!("[GDBLE] Core worker panicked during shutdown");
            }
        }
    }
}

impl Drop for CoreRuntime {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::test_support::FakeBackend;

    #[test]
    fn shutdown_completes_with_a_full_event_queue() {
        let mut runtime = CoreRuntime::spawn_with_backend(Arc::new(FakeBackend::default()));
        let client = runtime.client();
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while runtime.event_rx.len() < EVENT_CHANNEL_CAPACITY {
            for _ in 0..COMMAND_CHANNEL_CAPACITY {
                let _ = client.initialize();
            }
            assert!(
                std::time::Instant::now() < deadline,
                "event queue did not fill"
            );
            std::thread::sleep(Duration::from_millis(1));
        }
        let started = std::time::Instant::now();
        runtime.shutdown();
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn stopping_a_queued_scan_does_not_need_command_capacity() {
        let (command_tx, mut command_rx) = mpsc::channel(1);
        let client = CoreClient {
            command_tx,
            scan_control: Arc::new(ScanControl::default()),
            next_operation_id: Arc::new(AtomicI64::new(1)),
        };
        let id = client.start_scan(None).unwrap();
        assert_eq!(
            client.initialize().unwrap_err().error.unwrap().code,
            "QUEUE_FULL"
        );
        assert_eq!(client.stop_scan(id), id);
        let CoreCommand::StartScan { cancellation, .. } = command_rx.try_recv().unwrap() else {
            panic!("expected queued scan");
        };
        assert!(cancellation.is_cancelled());
    }
}
