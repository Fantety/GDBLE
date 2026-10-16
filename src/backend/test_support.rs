use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use tokio::sync::{broadcast, mpsc, Semaphore};

use super::*;

pub struct FakeBackend {
    state: Mutex<FakeState>,
    event_tx: Mutex<Option<mpsc::Sender<Result<BackendEvent, BleError>>>>,
    event_rx: Mutex<Option<mpsc::Receiver<Result<BackendEvent, BleError>>>>,
    notification_tx: Mutex<broadcast::Sender<Result<BackendNotification, BleError>>>,
    connect_gate: Semaphore,
}

#[derive(Clone, Copy)]
pub enum Fault {
    Hang,
    Panic,
}

#[derive(Default)]
struct FakeState {
    availability: Option<AdapterState>,
    disconnect_echo: bool,
    subscribe_notification: Option<Vec<u8>>,
    subscribe_error: Option<BleError>,
    devices: HashMap<DeviceId, DeviceInfo>,
    connected: HashSet<DeviceId>,
    fail_next_scan_start: Option<BleError>,
    fail_next_connect: Option<BleError>,
    fail_next_unsubscribe: Option<BleError>,
    notification_pumps_started: usize,
    block_next_connect: bool,
    write_modes: Vec<bool>,
    stop_scan_calls: usize,
    faults: HashMap<&'static str, Fault>,
}

impl Default for FakeBackend {
    fn default() -> Self {
        let (event_tx, event_rx) = mpsc::channel(64);
        let (notification_tx, _) = broadcast::channel(64);
        Self {
            state: Mutex::new(FakeState::default()),
            event_tx: Mutex::new(Some(event_tx)),
            event_rx: Mutex::new(Some(event_rx)),
            notification_tx: Mutex::new(notification_tx),
            connect_gate: Semaphore::new(0),
        }
    }
}

impl FakeBackend {
    pub fn echo_local_disconnect(&self) {
        self.state.lock().unwrap().disconnect_echo = true;
    }

    pub fn notify_on_subscribe(&self, value: Vec<u8>) {
        self.state.lock().unwrap().subscribe_notification = Some(value);
    }

    pub fn fail_subscribe(&self, error: BleError) {
        self.state.lock().unwrap().subscribe_error = Some(error);
    }

    pub fn emit_backend(&self, event: BackendEvent) {
        if let Some(sender) = self.event_tx.lock().unwrap().as_ref() {
            // Cleanup may disconnect after the adapter listener is dropped.
            if let Err(mpsc::error::TrySendError::Full(_)) = sender.try_send(Ok(event)) {
                panic!("fake adapter event producer exceeded its bounded capacity");
            }
        }
    }

    pub fn set_adapter_state(&self, state: AdapterState) {
        self.state.lock().unwrap().availability = Some(state.clone());
        self.emit_backend(BackendEvent::StateChanged(state));
    }

    pub fn emit_disconnect_echo(&self, address: &str) {
        self.emit_backend(BackendEvent::Disconnected(
            DeviceId::parse(address).unwrap(),
        ));
    }
    pub fn fault_next(&self, operation: &'static str, fault: Fault) {
        self.state.lock().unwrap().faults.insert(operation, fault);
    }

    async fn apply_fault(&self, operation: &str) {
        let fault = self.state.lock().unwrap().faults.remove(operation);
        match fault {
            Some(Fault::Hang) => futures::future::pending::<()>().await,
            Some(Fault::Panic) => panic!("injected backend panic in {operation}"),
            None => {}
        }
    }

    pub fn end_adapter_stream(&self) {
        self.event_tx.lock().unwrap().take();
    }

    pub fn end_notification_stream(&self) {
        *self.notification_tx.lock().unwrap() = broadcast::channel(64).0;
    }

    pub fn fail_next_scan_start(&self, error: BleError) {
        self.state.lock().unwrap().fail_next_scan_start = Some(error);
    }

    pub fn fail_next_unsubscribe(&self, error: BleError) {
        self.state.lock().unwrap().fail_next_unsubscribe = Some(error);
    }

    pub fn fail_next_connect(&self, error: BleError) {
        self.state.lock().unwrap().fail_next_connect = Some(error);
    }

    pub fn notification_pumps_started(&self) -> usize {
        self.state.lock().unwrap().notification_pumps_started
    }

    pub fn stop_scan_calls(&self) -> usize {
        self.state.lock().unwrap().stop_scan_calls
    }

    pub fn is_connected(&self, address: &str) -> bool {
        self.state
            .lock()
            .unwrap()
            .connected
            .contains(&DeviceId::parse(address).unwrap())
    }

    pub fn emit_notification(&self, characteristic_uuid: &str, value: Vec<u8>) {
        let _ = self
            .notification_tx
            .lock()
            .unwrap()
            .send(Ok(BackendNotification {
                characteristic_uuid: characteristic_uuid.to_ascii_lowercase(),
                value,
            }));
    }

    pub fn emit_notification_error(&self, error: BleError) {
        let _ = self.notification_tx.lock().unwrap().send(Err(error));
    }

    pub fn block_next_connect(&self) {
        self.state.lock().unwrap().block_next_connect = true;
    }

    pub fn release_connect(&self) {
        self.connect_gate.add_permits(1);
    }

    pub fn write_modes(&self) -> Vec<bool> {
        self.state.lock().unwrap().write_modes.clone()
    }

    pub fn add_device(&self, device: DeviceInfo) {
        let id = DeviceId::parse(&device.address).unwrap();
        self.state.lock().unwrap().devices.insert(id, device);
    }

    pub fn add_connected_device(&self, address: &str) {
        let id = DeviceId::parse(address).unwrap();
        let device = DeviceInfo::new(
            address.to_string(),
            None,
            None,
            Vec::new(),
            HashMap::new(),
            HashMap::new(),
            None,
        );
        let mut state = self.state.lock().unwrap();
        state.devices.insert(id.clone(), device);
        state.connected.insert(id);
    }

    pub fn emit_device_discovered(&self, address: &str) {
        let id = DeviceId::parse(address).unwrap();
        let device = self
            .state
            .lock()
            .unwrap()
            .devices
            .get(&id)
            .cloned()
            .unwrap();
        self.event_tx
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .try_send(Ok(BackendEvent::Discovered(device)))
            .unwrap();
    }

    pub fn emit_remote_disconnect(&self, address: &str) {
        let id = DeviceId::parse(address).unwrap();
        self.state.lock().unwrap().connected.remove(&id);
        self.event_tx
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .try_send(Ok(BackendEvent::Disconnected(id)))
            .unwrap();
    }

    fn ensure_connected(&self, device: &DeviceId) -> Result<(), BleError> {
        if self.state.lock().unwrap().connected.contains(device) {
            Ok(())
        } else {
            Err(BleError::NotConnected)
        }
    }
}

#[async_trait]
impl BleBackend for FakeBackend {
    async fn initialize(&self) -> Result<(AdapterInfo, BackendEventStream), BleError> {
        self.apply_fault("initialize").await;
        let receiver = match self.event_rx.lock().unwrap().take() {
            Some(receiver) => receiver,
            None => {
                let (sender, receiver) = mpsc::channel(64);
                *self.event_tx.lock().unwrap() = Some(sender);
                receiver
            }
        };
        Ok((
            AdapterInfo::new("Fake BLE Adapter".to_string(), None),
            Box::pin(tokio_stream(receiver)),
        ))
    }

    async fn adapter_state(&self) -> Result<AdapterState, BleError> {
        self.apply_fault("adapter_state").await;
        Ok(self
            .state
            .lock()
            .unwrap()
            .availability
            .clone()
            .unwrap_or(AdapterState::PoweredOn))
    }

    async fn device_info(&self, device: &DeviceId) -> Result<DeviceInfo, BleError> {
        self.apply_fault("snapshot").await;
        self.state
            .lock()
            .unwrap()
            .devices
            .get(device)
            .cloned()
            .ok_or_else(|| BleError::DeviceNotFound(device.as_str().to_string()))
    }

    async fn is_connected(&self, device: &DeviceId) -> Result<bool, BleError> {
        self.apply_fault("is_connected").await;
        Ok(self.state.lock().unwrap().connected.contains(device))
    }

    async fn start_scan(&self) -> Result<(), BleError> {
        self.apply_fault("start_scan").await;
        if let Some(error) = self.state.lock().unwrap().fail_next_scan_start.take() {
            Err(error)
        } else {
            Ok(())
        }
    }

    async fn stop_scan(&self) -> Result<(), BleError> {
        self.state.lock().unwrap().stop_scan_calls += 1;
        self.apply_fault("stop_scan").await;
        Ok(())
    }

    async fn connect(&self, device: &DeviceId) -> Result<(), BleError> {
        self.apply_fault("connect").await;
        if let Some(error) = self.state.lock().unwrap().fail_next_connect.take() {
            return Err(error);
        }
        let should_wait = {
            let mut state = self.state.lock().unwrap();
            std::mem::take(&mut state.block_next_connect)
        };
        if should_wait {
            self.connect_gate
                .acquire()
                .await
                .map_err(|error| BleError::ConnectionFailed(error.to_string()))?
                .forget();
        }
        let mut state = self.state.lock().unwrap();
        if !state.devices.contains_key(device) {
            return Err(BleError::DeviceNotFound(device.as_str().to_string()));
        }
        state.connected.insert(device.clone());
        Ok(())
    }

    async fn disconnect(&self, device: &DeviceId) -> Result<(), BleError> {
        self.apply_fault("disconnect").await;
        self.state.lock().unwrap().connected.remove(device);
        if self.state.lock().unwrap().disconnect_echo {
            self.emit_backend(BackendEvent::Disconnected(device.clone()));
        }
        self.apply_fault("disconnect_after_event").await;
        Ok(())
    }

    async fn discover_services(&self, device: &DeviceId) -> Result<Vec<BleServiceInfo>, BleError> {
        self.apply_fault("discover_services").await;
        self.ensure_connected(device)?;
        Ok(Vec::new())
    }

    async fn read(&self, device: &DeviceId, _key: &GattKey) -> Result<Vec<u8>, BleError> {
        self.apply_fault("read").await;
        self.ensure_connected(device)?;
        Ok(Vec::new())
    }

    async fn write(
        &self,
        device: &DeviceId,
        _key: &GattKey,
        _data: &[u8],
        with_response: bool,
    ) -> Result<(), BleError> {
        self.apply_fault("write").await;
        self.ensure_connected(device)?;
        self.state.lock().unwrap().write_modes.push(with_response);
        Ok(())
    }

    async fn subscribe(&self, device: &DeviceId, _key: &GattKey) -> Result<(), BleError> {
        self.apply_fault("subscribe").await;
        let value = self.state.lock().unwrap().subscribe_notification.take();
        if let Some(value) = value {
            self.emit_notification(_key.characteristic_uuid(), value);
        }
        self.apply_fault("subscribe_after_notification").await;
        let error = self.state.lock().unwrap().subscribe_error.take();
        if let Some(error) = error {
            return Err(error);
        }
        self.ensure_connected(device)
    }

    async fn unsubscribe(&self, device: &DeviceId, _key: &GattKey) -> Result<(), BleError> {
        self.apply_fault("unsubscribe").await;
        self.ensure_connected(device)?;
        if let Some(error) = self.state.lock().unwrap().fail_next_unsubscribe.take() {
            Err(error)
        } else {
            Ok(())
        }
    }

    async fn notifications(&self, device: &DeviceId) -> Result<NotificationStream, BleError> {
        self.apply_fault("notifications").await;
        self.ensure_connected(device)?;
        self.state.lock().unwrap().notification_pumps_started += 1;
        let receiver = self.notification_tx.lock().unwrap().subscribe();
        Ok(Box::pin(broadcast_stream(receiver)))
    }
}

fn tokio_stream<T: Send + 'static>(
    receiver: mpsc::Receiver<T>,
) -> impl Stream<Item = T> + Send + 'static {
    futures::stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|item| (item, receiver))
    })
}

fn broadcast_stream(
    receiver: broadcast::Receiver<Result<BackendNotification, BleError>>,
) -> impl Stream<Item = Result<BackendNotification, BleError>> + Send + 'static {
    futures::stream::unfold(receiver, |mut receiver| async move {
        match receiver.recv().await {
            Ok(item) => Some((item, receiver)),
            Err(broadcast::error::RecvError::Lagged(_)) => Some((
                Err(BleError::EventOverflow("backend_notifications".into())),
                receiver,
            )),
            Err(broadcast::error::RecvError::Closed) => None,
        }
    })
}
