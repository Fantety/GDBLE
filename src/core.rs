use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use futures::{FutureExt, StreamExt};
use std::future::Future;
use std::panic::AssertUnwindSafe;
use tokio::sync::mpsc;
use tokio::task::{AbortHandle, JoinSet};
use tokio_util::sync::CancellationToken;

use crate::backend::{
    BackendEvent, BackendEventStream, BackendNotification, BleBackend, BtleplugBackend,
};
use crate::ble_service::BleServiceInfo;
pub use crate::types::{AdapterInfo, BleError, DeviceInfo};

pub const COMMAND_CHANNEL_CAPACITY: usize = 64;
pub const EVENT_CHANNEL_CAPACITY: usize = 1024;
const INTERNAL_CHANNEL_CAPACITY: usize = 256;

mod client;
mod protocol;
use client::ScanControl;
pub use client::{CoreClient, CoreRuntime, Submission};
pub use protocol::*;

#[derive(Clone, Copy)]
struct CoreOptions {
    operation_timeout: Duration,
    shutdown_timeout: Duration,
}

impl Default for CoreOptions {
    fn default() -> Self {
        Self {
            operation_timeout: Duration::from_secs(30),
            shutdown_timeout: Duration::from_secs(2),
        }
    }
}

async fn guarded<T>(
    name: &str,
    deadline: Duration,
    future: impl Future<Output = Result<T, BleError>>,
) -> Result<T, BleError> {
    match tokio::time::timeout(deadline, AssertUnwindSafe(future).catch_unwind()).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err(BleError::InternalError(format!("{name} panicked"))),
        Err(_) => Err(BleError::Timeout(name.to_string())),
    }
}

enum CoreCommand {
    Initialize {
        operation_id: OperationId,
    },
    StartScan {
        operation_id: OperationId,
        timeout: Option<Duration>,
        cancellation: CancellationToken,
    },
    Device {
        operation_id: OperationId,
        device: DeviceId,
        action: DeviceAction,
    },
}

#[derive(Clone, Debug)]
enum DeviceAction {
    Connect,
    Disconnect,
    DiscoverServices,
    Read(GattKey),
    Write {
        key: GattKey,
        data: Vec<u8>,
        with_response: bool,
    },
    Subscribe(GattKey),
    Unsubscribe(GattKey),
}

impl DeviceAction {
    fn operation(&self) -> Operation {
        match self {
            Self::Connect => Operation::Connect,
            Self::Disconnect => Operation::Disconnect,
            Self::DiscoverServices => Operation::DiscoverServices,
            Self::Read(_) => Operation::Read,
            Self::Write { .. } => Operation::Write,
            Self::Subscribe(_) => Operation::Subscribe,
            Self::Unsubscribe(_) => Operation::Unsubscribe,
        }
    }

    fn context(&self, device: &DeviceId) -> EventContext {
        match self {
            Self::Read(key)
            | Self::Write { key, .. }
            | Self::Subscribe(key)
            | Self::Unsubscribe(key) => EventContext::for_gatt(device, key),
            _ => EventContext::for_device(device),
        }
    }
}

enum InternalEvent {
    Initialized {
        operation_id: OperationId,
        result: Result<(AdapterInfo, BackendEventStream), BleError>,
    },
    Backend {
        generation: u64,
        result: Result<BackendEvent, BleError>,
    },
    AdapterStreamEnded {
        generation: u64,
        error: BleError,
    },
    ScanStarted {
        operation_id: OperationId,
        generation: ScanGeneration,
        result: Result<(), BleError>,
    },
    ScanTimedOut {
        generation: ScanGeneration,
    },
    ScanStopped {
        operation_id: OperationId,
        generation: ScanGeneration,
        reason: String,
        result: Result<(), BleError>,
    },
    DeviceCompleted {
        operation_id: OperationId,
        device: DeviceId,
        generation: u64,
        action: DeviceAction,
        result: Result<EventData, BleError>,
    },
    Notification {
        device: DeviceId,
        generation: u64,
        notification: BackendNotification,
    },
    NotificationPumpFailed {
        device: DeviceId,
        generation: u64,
        error: BleError,
    },
    NotificationPumpEnded {
        device: DeviceId,
        generation: u64,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScanStatus {
    Idle,
    Starting,
    Scanning,
    Stopping,
}

struct ScanRuntimeState {
    tracker: ScanSession,
    status: ScanStatus,
    cancellation: Option<CancellationToken>,
    stop_requested: bool,
}

impl Default for ScanRuntimeState {
    fn default() -> Self {
        Self {
            tracker: ScanSession::default(),
            status: ScanStatus::Idle,
            cancellation: None,
            stop_requested: false,
        }
    }
}

struct InFlightOperation {
    operation_id: OperationId,
    generation: u64,
    action: DeviceAction,
    abort_handle: AbortHandle,
}

struct QueuedOperation {
    operation_id: OperationId,
    action: DeviceAction,
}

struct DeviceSession {
    state: ConnectionState,
    generation: u64,
    in_flight: Option<InFlightOperation>,
    pending: VecDeque<QueuedOperation>,
    services: Vec<BleServiceInfo>,
    subscriptions: HashSet<GattKey>,
    notification_cancellation: Option<CancellationToken>,
    notification_generation: u64,
}

impl Default for DeviceSession {
    fn default() -> Self {
        Self {
            state: ConnectionState::Disconnected,
            generation: 0,
            in_flight: None,
            pending: VecDeque::new(),
            services: Vec::new(),
            subscriptions: HashSet::new(),
            notification_cancellation: None,
            notification_generation: 0,
        }
    }
}

struct CoreState {
    backend: Arc<dyn BleBackend>,
    event_tx: mpsc::Sender<BleEvent>,
    internal_tx: mpsc::Sender<InternalEvent>,
    initialized: bool,
    initializing: bool,
    adapter_info: Option<AdapterInfo>,
    scan: ScanRuntimeState,
    known_devices: HashMap<DeviceId, DeviceInfo>,
    devices: HashMap<DeviceId, DeviceSession>,
    shutdown: CancellationToken,
    scan_control: Arc<ScanControl>,
    options: CoreOptions,
    adapter_generation: u64,
}

async fn run_core(
    backend: Arc<dyn BleBackend>,
    mut command_rx: mpsc::Receiver<CoreCommand>,
    event_tx: mpsc::Sender<BleEvent>,
    scan_control: Arc<ScanControl>,
    shutdown: CancellationToken,
    options: CoreOptions,
) {
    let (internal_tx, mut internal_rx) = mpsc::channel(INTERNAL_CHANNEL_CAPACITY);
    let mut state = CoreState {
        backend,
        event_tx,
        internal_tx,
        initialized: false,
        initializing: false,
        adapter_info: None,
        scan: ScanRuntimeState::default(),
        known_devices: HashMap::new(),
        devices: HashMap::new(),
        shutdown: shutdown.clone(),
        scan_control: scan_control.clone(),
        options,
        adapter_generation: 0,
    };
    let mut tasks = JoinSet::new();
    loop {
        tokio::select! {
            () = shutdown.cancelled() => {
                state.shutdown(&mut tasks).await;
                break;
            }
            command = command_rx.recv() => {
                match command {
                    None => {
                        state.shutdown(&mut tasks).await;
                        break;
                    }
                    Some(command) => state.handle_command(command, &mut tasks).await,
                }
            }
            () = scan_control.changed() => {
                if let Some(active) = state.scan.tracker.active() {
                    if state.scan.cancellation.as_ref().is_some_and(CancellationToken::is_cancelled) {
                        state.stop_scan(active.operation_id, "stopped", &mut tasks).await;
                    }
                }
            }
            internal = internal_rx.recv() => {
                if let Some(internal) = internal {
                    state.handle_internal(internal, &mut tasks).await;
                }
            }
            result = tasks.join_next(), if !tasks.is_empty() => {
                if let Some(Err(error)) = result {
                    eprintln!("[GDBLE] Background task failed: {error}");
                }
            }
        }
    }
}

impl CoreState {
    async fn handle_command(&mut self, command: CoreCommand, tasks: &mut JoinSet<()>) {
        match command {
            CoreCommand::Initialize { operation_id } => self.initialize(operation_id, tasks).await,
            CoreCommand::StartScan {
                operation_id,
                timeout,
                cancellation,
            } => {
                self.start_scan(operation_id, timeout, cancellation, tasks)
                    .await;
            }
            CoreCommand::Device {
                operation_id,
                device,
                action,
            } => {
                self.device_command(operation_id, device, action, tasks)
                    .await;
            }
        }
    }

    async fn initialize(&mut self, operation_id: OperationId, tasks: &mut JoinSet<()>) {
        if self.initialized {
            let data = self
                .adapter_info
                .clone()
                .map(EventData::Adapter)
                .unwrap_or_default();
            self.emit(BleEvent::succeeded(
                Operation::Initialize,
                operation_id,
                EventContext::default(),
                data,
            ))
            .await;
            return;
        }
        if self.initializing {
            self.emit(BleEvent::failed(
                Operation::Initialize,
                operation_id,
                EventContext::default(),
                BleError::Busy("initialization already in progress".to_string()),
            ))
            .await;
            return;
        }
        self.initializing = true;
        self.emit(BleEvent::started(
            Operation::Initialize,
            operation_id,
            EventContext::default(),
        ))
        .await;
        let backend = self.backend.clone();
        let internal_tx = self.internal_tx.clone();
        let deadline = self.options.operation_timeout;
        tasks.spawn(async move {
            let result = guarded("initialize", deadline, backend.initialize()).await;
            let _ = internal_tx
                .send(InternalEvent::Initialized {
                    operation_id,
                    result,
                })
                .await;
        });
    }

    async fn start_scan(
        &mut self,
        operation_id: OperationId,
        timeout: Option<Duration>,
        cancellation: CancellationToken,
        tasks: &mut JoinSet<()>,
    ) {
        if !self.initialized {
            self.emit(BleEvent::failed(
                Operation::Scan,
                operation_id,
                EventContext::default(),
                BleError::NotInitialized,
            ))
            .await;
            return;
        }
        let generation = match self.scan.tracker.start(operation_id, timeout) {
            Ok(generation) => generation,
            Err(error) => {
                self.emit(BleEvent::failed(
                    Operation::Scan,
                    operation_id,
                    EventContext::default(),
                    error,
                ))
                .await;
                return;
            }
        };
        self.scan.status = ScanStatus::Starting;
        self.scan.stop_requested = cancellation.is_cancelled();
        self.scan.cancellation = Some(cancellation);
        let backend = self.backend.clone();
        let internal_tx = self.internal_tx.clone();
        let deadline = self.options.operation_timeout;
        tasks.spawn(async move {
            let result = guarded("start_scan", deadline, backend.start_scan()).await;
            let _ = internal_tx
                .send(InternalEvent::ScanStarted {
                    operation_id,
                    generation,
                    result,
                })
                .await;
        });
    }

    async fn stop_scan(
        &mut self,
        operation_id: OperationId,
        reason: &str,
        tasks: &mut JoinSet<()>,
    ) {
        let Some(active) = self.scan.tracker.active() else {
            return;
        };
        if active.operation_id != operation_id {
            return;
        }
        if let Some(cancellation) = self.scan.cancellation.as_ref() {
            cancellation.cancel();
        }
        match self.scan.status {
            ScanStatus::Starting => self.scan.stop_requested = true,
            ScanStatus::Scanning => self.spawn_scan_stop(active, reason.to_string(), tasks),
            ScanStatus::Stopping | ScanStatus::Idle => {}
        }
    }

    fn spawn_scan_stop(&mut self, active: ActiveScan, reason: String, tasks: &mut JoinSet<()>) {
        if self.scan.status == ScanStatus::Stopping {
            return;
        }
        self.scan.status = ScanStatus::Stopping;
        let backend = self.backend.clone();
        let internal_tx = self.internal_tx.clone();
        let deadline = self.options.operation_timeout;
        tasks.spawn(async move {
            let result = guarded("stop_scan", deadline, backend.stop_scan()).await;
            let _ = internal_tx
                .send(InternalEvent::ScanStopped {
                    operation_id: active.operation_id,
                    generation: active.generation,
                    reason,
                    result,
                })
                .await;
        });
    }

    async fn device_command(
        &mut self,
        operation_id: OperationId,
        device: DeviceId,
        action: DeviceAction,
        tasks: &mut JoinSet<()>,
    ) {
        if !self.initialized {
            self.emit(BleEvent::failed(
                action.operation(),
                operation_id,
                action.context(&device),
                BleError::NotInitialized,
            ))
            .await;
            return;
        }
        let session = self.devices.entry(device.clone()).or_default();
        if session.pending.len() >= COMMAND_CHANNEL_CAPACITY {
            self.emit(BleEvent::failed(
                action.operation(),
                operation_id,
                action.context(&device),
                BleError::QueueFull,
            ))
            .await;
            return;
        }
        session.pending.push_back(QueuedOperation {
            operation_id,
            action,
        });
        self.start_next_device_action(&device, tasks).await;
    }

    async fn start_next_device_action(&mut self, device: &DeviceId, tasks: &mut JoinSet<()>) {
        loop {
            let Some(queued) = self.devices.get_mut(device).and_then(|session| {
                if session.in_flight.is_some() {
                    None
                } else {
                    session.pending.pop_front()
                }
            }) else {
                return;
            };
            let operation_id = queued.operation_id;
            let action = queued.action;
            let restart_notification_pump = matches!(&action, DeviceAction::Subscribe(key) if {
                let session = self.devices.get(device).expect("device session must exist");
                session.subscriptions.contains(key)
                    && session.notification_cancellation.is_none()
            });
            if let Some(result) = self.validate_device_action(device, &action) {
                match result {
                    Ok(data) => {
                        self.emit(BleEvent::succeeded(
                            action.operation(),
                            operation_id,
                            action.context(device),
                            data,
                        ))
                        .await;
                    }
                    Err(error) => {
                        self.emit(BleEvent::failed(
                            action.operation(),
                            operation_id,
                            action.context(device),
                            error,
                        ))
                        .await;
                    }
                }
                if restart_notification_pump {
                    self.ensure_notification_pump(device, tasks).await;
                }
                continue;
            }

            let session = self
                .devices
                .get_mut(device)
                .expect("device session must exist");
            session.generation = session.generation.wrapping_add(1);
            let generation = session.generation;
            session.state = match &action {
                DeviceAction::Connect => ConnectionState::Connecting,
                DeviceAction::Disconnect => ConnectionState::Disconnecting,
                _ => session.state,
            };
            let operation = action.operation();
            let context = action.context(device);
            self.emit(BleEvent::started(operation, operation_id, context))
                .await;
            let backend = self.backend.clone();
            let internal_tx = self.internal_tx.clone();
            let device_for_task = device.clone();
            let action_for_task = action.clone();
            let deadline = self.options.operation_timeout;
            let abort_handle = tasks.spawn(async move {
                let result = guarded(
                    operation.as_str(),
                    deadline,
                    execute_device_action(backend.as_ref(), &device_for_task, &action_for_task),
                )
                .await;
                let _ = internal_tx
                    .send(InternalEvent::DeviceCompleted {
                        operation_id,
                        device: device_for_task,
                        generation,
                        action: action_for_task,
                        result,
                    })
                    .await;
            });
            self.devices
                .get_mut(device)
                .expect("device session must exist")
                .in_flight = Some(InFlightOperation {
                operation_id,
                generation,
                action,
                abort_handle,
            });
            return;
        }
    }

    fn validate_device_action(
        &self,
        device: &DeviceId,
        action: &DeviceAction,
    ) -> Option<Result<EventData, BleError>> {
        let session = self.devices.get(device).expect("device session must exist");
        let state = session.state;
        if matches!(action, DeviceAction::Connect) && state == ConnectionState::Connected {
            return Some(Ok(EventData::Reason("already_connected".to_string())));
        }
        if matches!(action, DeviceAction::Disconnect) && state == ConnectionState::Disconnected {
            return Some(Ok(EventData::Reason("already_disconnected".to_string())));
        }
        if !matches!(action, DeviceAction::Connect | DeviceAction::Disconnect)
            && state != ConnectionState::Connected
        {
            return Some(Err(BleError::NotConnected));
        }
        if let DeviceAction::Subscribe(key) = &action {
            let subscriptions = &session.subscriptions;
            if subscriptions.contains(key) {
                return Some(Ok(EventData::Reason("already_subscribed".to_string())));
            }
            if subscriptions.iter().any(|existing| {
                existing.characteristic_uuid() == key.characteristic_uuid()
                    && existing.service_uuid() != key.service_uuid()
            }) {
                return Some(Err(BleError::AmbiguousCharacteristic(
                    key.characteristic_uuid().to_string(),
                )));
            }
        }
        None
    }

    async fn handle_internal(&mut self, event: InternalEvent, tasks: &mut JoinSet<()>) {
        match event {
            InternalEvent::Initialized {
                operation_id,
                result,
            } => self.initialized(operation_id, result, tasks).await,
            InternalEvent::Backend { generation, result } => {
                if self.initialized && generation == self.adapter_generation {
                    self.backend_event(result, tasks).await;
                }
            }
            InternalEvent::AdapterStreamEnded { generation, error } => {
                self.adapter_stream_ended(generation, error, tasks).await;
            }
            InternalEvent::ScanStarted {
                operation_id,
                generation,
                result,
            } => {
                self.scan_started(operation_id, generation, result, tasks)
                    .await
            }
            InternalEvent::ScanTimedOut { generation } => {
                if let Some(active) = self.scan.tracker.active() {
                    if active.generation == generation {
                        self.stop_scan(active.operation_id, "timeout", tasks).await;
                    }
                }
            }
            InternalEvent::ScanStopped {
                operation_id,
                generation,
                reason,
                result,
            } => {
                self.scan_stopped(operation_id, generation, reason, result)
                    .await
            }
            InternalEvent::DeviceCompleted {
                operation_id,
                device,
                generation,
                action,
                result,
            } => {
                self.device_completed(operation_id, device, generation, action, result, tasks)
                    .await
            }
            InternalEvent::Notification {
                device,
                generation,
                notification,
            } => {
                self.notification(device, generation, notification).await;
            }
            InternalEvent::NotificationPumpFailed {
                device,
                generation,
                error,
            } => {
                if self.finish_notification_pump(&device, generation) {
                    self.notification_failure(&device, error).await;
                }
            }
            InternalEvent::NotificationPumpEnded { device, generation } => {
                if self.finish_notification_pump(&device, generation) {
                    self.notification_failure(
                        &device,
                        BleError::OperationFailed(
                            "notification stream ended; subscribe again to restart it".to_string(),
                        ),
                    )
                    .await;
                }
            }
        }
    }

    async fn initialized(
        &mut self,
        operation_id: OperationId,
        result: Result<(AdapterInfo, BackendEventStream), BleError>,
        tasks: &mut JoinSet<()>,
    ) {
        self.initializing = false;
        match result {
            Ok((adapter_info, mut stream)) => {
                self.initialized = true;
                self.adapter_info = Some(adapter_info.clone());
                self.emit(BleEvent::succeeded(
                    Operation::Initialize,
                    operation_id,
                    EventContext::default(),
                    EventData::Adapter(adapter_info),
                ))
                .await;
                self.adapter_generation = self.adapter_generation.wrapping_add(1);
                let generation = self.adapter_generation;
                let internal_tx = self.internal_tx.clone();
                let shutdown = self.shutdown.clone();
                tasks.spawn(async move {
                    loop {
                        let next = tokio::select! {
                            () = shutdown.cancelled() => return,
                            next = AssertUnwindSafe(stream.next()).catch_unwind() => next,
                        };
                        let error = match next {
                            Ok(Some(result)) => {
                                if internal_tx
                                    .send(InternalEvent::Backend { generation, result })
                                    .await
                                    .is_err()
                                {
                                    return;
                                }
                                continue;
                            }
                            Ok(None) => BleError::OperationFailed(
                                "adapter event stream ended; initialize again".to_string(),
                            ),
                            Err(_) => {
                                BleError::InternalError("adapter event stream panicked".to_string())
                            }
                        };
                        let _ = internal_tx
                            .send(InternalEvent::AdapterStreamEnded { generation, error })
                            .await;
                        return;
                    }
                });
            }
            Err(error) => {
                self.emit(BleEvent::failed(
                    Operation::Initialize,
                    operation_id,
                    EventContext::default(),
                    error,
                ))
                .await
            }
        }
    }

    async fn scan_started(
        &mut self,
        operation_id: OperationId,
        generation: ScanGeneration,
        result: Result<(), BleError>,
        tasks: &mut JoinSet<()>,
    ) {
        if self.scan.tracker.active_generation() != Some(generation) {
            return;
        }
        match result {
            Ok(()) => {
                self.scan.status = ScanStatus::Scanning;
                self.emit(BleEvent::started(
                    Operation::Scan,
                    operation_id,
                    EventContext::default(),
                ))
                .await;
                if self.scan.stop_requested
                    || self
                        .scan
                        .cancellation
                        .as_ref()
                        .is_some_and(CancellationToken::is_cancelled)
                {
                    if let Some(active) = self.scan.tracker.active() {
                        self.spawn_scan_stop(active, "stopped".to_string(), tasks);
                    }
                } else if let Some(active) = self.scan.tracker.active() {
                    if let Some(timeout) = active.timeout {
                        let cancellation = self.scan.cancellation.clone().unwrap_or_default();
                        let internal_tx = self.internal_tx.clone();
                        tasks.spawn(async move {
                            tokio::select! {
                                () = tokio::time::sleep(timeout) => { let _ = internal_tx.send(InternalEvent::ScanTimedOut { generation }).await; }
                                () = cancellation.cancelled() => {}
                            }
                        });
                    }
                }
            }
            Err(error) => {
                let _ = self.scan.tracker.finish(generation);
                self.scan.status = ScanStatus::Idle;
                self.scan.cancellation = None;
                self.scan.stop_requested = false;
                self.emit(BleEvent::failed(
                    Operation::Scan,
                    operation_id,
                    EventContext::default(),
                    error,
                ))
                .await;
            }
        }
    }

    async fn scan_stopped(
        &mut self,
        operation_id: OperationId,
        generation: ScanGeneration,
        reason: String,
        result: Result<(), BleError>,
    ) {
        if !self.scan.tracker.finish(generation).unwrap_or(false) {
            return;
        }
        self.scan.status = ScanStatus::Idle;
        self.scan.cancellation = None;
        self.scan.stop_requested = false;
        match result {
            Ok(()) => {
                self.emit(BleEvent::succeeded(
                    Operation::Scan,
                    operation_id,
                    EventContext::default(),
                    EventData::Reason(reason),
                ))
                .await
            }
            Err(error) => {
                self.emit(BleEvent::failed(
                    Operation::Scan,
                    operation_id,
                    EventContext::default(),
                    error,
                ))
                .await
            }
        }
    }

    async fn backend_event(
        &mut self,
        result: Result<BackendEvent, BleError>,
        tasks: &mut JoinSet<()>,
    ) {
        match result {
            Ok(BackendEvent::Discovered(info)) => self.device_progress(info).await,
            Ok(BackendEvent::Updated(info)) => self.device_progress(info).await,
            Ok(BackendEvent::Disconnected(device)) => self.remote_disconnect(device, tasks).await,
            Err(error) => {
                let operation_id = self
                    .scan
                    .tracker
                    .active()
                    .map(|active| active.operation_id)
                    .unwrap_or(OperationId::UNSOLICITED);
                self.emit(BleEvent {
                    operation: Operation::Scan,
                    phase: Phase::Failed,
                    operation_id,
                    terminal: false,
                    context: EventContext::default(),
                    data: EventData::None,
                    error: Some(error.into()),
                    connection_state: None,
                    adapter_initialized: None,
                })
                .await;
            }
        }
    }

    async fn device_progress(&mut self, info: DeviceInfo) {
        let Ok(device) = DeviceId::parse(&info.address) else {
            return;
        };
        let kind = match self.known_devices.get(&device) {
            None => DeviceProgressKind::Discovered,
            Some(previous) if previous == &info => return,
            Some(_) => DeviceProgressKind::Updated,
        };
        let context = EventContext::for_device(&device);
        self.known_devices.insert(device, info.clone());
        let operation_id = self
            .scan
            .tracker
            .active()
            .map(|active| active.operation_id)
            .unwrap_or(OperationId::UNSOLICITED);
        self.emit(BleEvent::progress(
            operation_id,
            context,
            EventData::Device { kind, info },
        ))
        .await;
    }

    async fn remote_disconnect(&mut self, device: DeviceId, _tasks: &mut JoinSet<()>) {
        let mut cancelled = None;
        let mut cancelled_pending = Vec::new();
        if let Some(session) = self.devices.get_mut(&device) {
            session.state = ConnectionState::Disconnected;
            session.generation = session.generation.wrapping_add(1);
            if let Some(in_flight) = session.in_flight.take() {
                in_flight.abort_handle.abort();
                cancelled = Some((
                    in_flight.action.operation(),
                    in_flight.operation_id,
                    in_flight.action.context(&device),
                ));
            }
            if let Some(cancellation) = session.notification_cancellation.take() {
                cancellation.cancel();
            }
            session.notification_generation = session.notification_generation.wrapping_add(1);
            session.subscriptions.clear();
            session.services.clear();
            cancelled_pending.extend(session.pending.drain(..));
        }
        if let Some((operation, operation_id, context)) = cancelled {
            self.emit(BleEvent::failed(
                operation,
                operation_id,
                context,
                BleError::Cancelled("device disconnected remotely".to_string()),
            ))
            .await;
        }
        for queued in cancelled_pending {
            self.emit(BleEvent::failed(
                queued.action.operation(),
                queued.operation_id,
                queued.action.context(&device),
                BleError::Cancelled("device disconnected remotely".to_string()),
            ))
            .await;
        }
        self.emit(BleEvent::received(
            Operation::Disconnect,
            EventContext::for_device(&device),
            EventData::Reason("remote".to_string()),
        ))
        .await;
    }

    async fn device_completed(
        &mut self,
        operation_id: OperationId,
        device: DeviceId,
        generation: u64,
        action: DeviceAction,
        result: Result<EventData, BleError>,
        tasks: &mut JoinSet<()>,
    ) {
        let operation = action.operation();
        let context = action.context(&device);
        let mut start_notification_pump = false;
        let event = match result {
            Ok(data) => {
                {
                    let Some(session) = self.devices.get_mut(&device) else {
                        return;
                    };
                    let matches_current = session.in_flight.as_ref().is_some_and(|in_flight| {
                        in_flight.operation_id == operation_id && in_flight.generation == generation
                    });
                    if !matches_current {
                        return;
                    }
                    session.in_flight = None;
                    match &action {
                        DeviceAction::Connect => session.state = ConnectionState::Connected,
                        DeviceAction::Disconnect => {
                            session.state = ConnectionState::Disconnected;
                            if let Some(cancellation) = session.notification_cancellation.take() {
                                cancellation.cancel();
                            }
                            session.notification_generation =
                                session.notification_generation.wrapping_add(1);
                            session.subscriptions.clear();
                            session.services.clear();
                        }
                        DeviceAction::DiscoverServices => {
                            if let EventData::Services(services) = &data {
                                session.services = services.clone();
                            }
                        }
                        DeviceAction::Subscribe(key) => {
                            session.subscriptions.insert(key.clone());
                            start_notification_pump = true;
                        }
                        DeviceAction::Unsubscribe(key) => {
                            session.subscriptions.remove(key);
                        }
                        DeviceAction::Read(_) | DeviceAction::Write { .. } => {}
                    }
                }
                BleEvent::succeeded(operation, operation_id, context, data)
            }
            Err(error) => {
                {
                    let Some(session) = self.devices.get_mut(&device) else {
                        return;
                    };
                    let matches_current = session.in_flight.as_ref().is_some_and(|in_flight| {
                        in_flight.operation_id == operation_id && in_flight.generation == generation
                    });
                    if !matches_current {
                        return;
                    }
                    session.in_flight = None;
                    match &action {
                        DeviceAction::Connect => session.state = ConnectionState::Disconnected,
                        DeviceAction::Disconnect => session.state = ConnectionState::Connected,
                        _ => {}
                    }
                }
                BleEvent::failed(operation, operation_id, context, error)
            }
        };
        self.emit(event).await;
        if start_notification_pump {
            self.ensure_notification_pump(&device, tasks).await;
        }
        self.start_next_device_action(&device, tasks).await;
    }

    async fn ensure_notification_pump(&mut self, device: &DeviceId, tasks: &mut JoinSet<()>) {
        let Some(session) = self.devices.get_mut(device) else {
            return;
        };
        if session.subscriptions.is_empty() || session.notification_cancellation.is_some() {
            return;
        }
        session.notification_generation = session.notification_generation.wrapping_add(1);
        let generation = session.notification_generation;
        let cancellation = CancellationToken::new();
        session.notification_cancellation = Some(cancellation.clone());
        let backend = self.backend.clone();
        let internal_tx = self.internal_tx.clone();
        let device = device.clone();
        let deadline = self.options.operation_timeout;
        tasks.spawn(async move {
            let mut stream =
                match guarded("notifications", deadline, backend.notifications(&device)).await {
                    Ok(stream) => stream,
                    Err(error) => {
                        let _ = internal_tx
                            .send(InternalEvent::NotificationPumpFailed {
                                device,
                                generation,
                                error,
                            })
                            .await;
                        return;
                    }
                };
            loop {
                let next = tokio::select! {
                    () = cancellation.cancelled() => return,
                    next = AssertUnwindSafe(stream.next()).catch_unwind() => next,
                };
                let event = match next {
                    Ok(Some(Ok(notification))) => InternalEvent::Notification {
                        device: device.clone(),
                        generation,
                        notification,
                    },
                    Ok(Some(Err(error))) => InternalEvent::NotificationPumpFailed {
                        device: device.clone(),
                        generation,
                        error,
                    },
                    Err(_) => InternalEvent::NotificationPumpFailed {
                        device: device.clone(),
                        generation,
                        error: BleError::InternalError("notification stream panicked".to_string()),
                    },
                    Ok(None) => InternalEvent::NotificationPumpEnded {
                        device: device.clone(),
                        generation,
                    },
                };
                let ended = !matches!(event, InternalEvent::Notification { .. });
                if internal_tx.send(event).await.is_err() || ended {
                    return;
                }
            }
        });
    }

    fn finish_notification_pump(&mut self, device: &DeviceId, generation: u64) -> bool {
        self.devices.get_mut(device).is_some_and(|session| {
            session.notification_generation == generation
                && session.notification_cancellation.take().is_some()
        })
    }

    async fn notification_failure(&self, device: &DeviceId, error: BleError) {
        let mut event = BleEvent::failed(
            Operation::Notification,
            OperationId::UNSOLICITED,
            EventContext::for_device(device),
            error,
        );
        event.terminal = false;
        self.emit(event).await;
    }

    async fn notification(
        &mut self,
        device: DeviceId,
        generation: u64,
        notification: BackendNotification,
    ) {
        let Some(session) = self.devices.get(&device) else {
            return;
        };
        if session.notification_generation != generation
            || session.state != ConnectionState::Connected
            || session.notification_cancellation.is_none()
        {
            return;
        }
        let characteristic_uuid = notification.characteristic_uuid.to_ascii_lowercase();
        let Some(key) = session
            .subscriptions
            .iter()
            .find(|key| key.characteristic_uuid() == characteristic_uuid)
        else {
            return;
        };
        let event = BleEvent::received(
            Operation::Notification,
            EventContext::for_gatt(&device, key),
            EventData::Bytes(notification.value),
        );
        self.emit(event).await;
    }

    async fn adapter_stream_ended(
        &mut self,
        generation: u64,
        error: BleError,
        tasks: &mut JoinSet<()>,
    ) {
        if generation != self.adapter_generation || !self.initialized {
            return;
        }
        self.initialized = false;
        self.adapter_info = None;
        self.known_devices.clear();
        let active_scan = self.scan.tracker.active();
        let connected_devices = self.cleanup_devices();
        tasks.abort_all();
        // Finish old backend work before accepting a new adapter session.
        self.cleanup_backend(tasks, active_scan.is_some(), &connected_devices)
            .await;
        if let Some(active) = self.scan.tracker.active() {
            if let Some(token) = self.scan.cancellation.take() {
                token.cancel();
            }
            let _ = self.scan.tracker.finish(active.generation);
            self.scan.status = ScanStatus::Idle;
            self.scan.stop_requested = false;
            self.emit(BleEvent::failed(
                Operation::Scan,
                active.operation_id,
                EventContext::default(),
                BleError::Cancelled("adapter event stream ended".to_string()),
            ))
            .await;
        }
        let devices: Vec<_> = self.devices.keys().cloned().collect();
        for device in devices {
            self.remote_disconnect(device, tasks).await;
        }
        let mut event = BleEvent::failed(
            Operation::Initialize,
            OperationId::UNSOLICITED,
            EventContext::default(),
            error,
        );
        event.terminal = false;
        self.emit(event).await;
    }

    async fn shutdown(&mut self, tasks: &mut JoinSet<()>) {
        let scan_active = self.scan.tracker.active().is_some();
        let devices = self.cleanup_devices();
        if let Some(token) = self.scan.cancellation.take() {
            token.cancel();
        }
        for session in self.devices.values_mut() {
            if let Some(operation) = session.in_flight.take() {
                operation.abort_handle.abort();
            }
            if let Some(token) = session.notification_cancellation.take() {
                token.cancel();
            }
            session.pending.clear();
        }
        tasks.abort_all();
        self.cleanup_backend(tasks, scan_active, &devices).await;
    }

    fn cleanup_devices(&self) -> Vec<DeviceId> {
        self.devices
            .iter()
            .filter(|(_, session)| session.state != ConnectionState::Disconnected)
            .map(|(device, _)| device.clone())
            .collect()
    }

    async fn cleanup_backend(
        &self,
        tasks: &mut JoinSet<()>,
        scan_active: bool,
        devices: &[DeviceId],
    ) {
        let budget = self.options.shutdown_timeout;
        let _ = tokio::time::timeout(budget, async {
            while tasks.join_next().await.is_some() {}
            let stop = async {
                if scan_active {
                    let _ = guarded("cleanup scan", budget, self.backend.stop_scan()).await;
                }
            };
            let disconnect = futures::future::join_all(devices.iter().map(|device| {
                guarded(
                    "cleanup disconnect",
                    budget,
                    self.backend.disconnect(device),
                )
            }));
            tokio::join!(stop, disconnect);
        })
        .await;
    }

    async fn emit(&self, mut event: BleEvent) {
        if event.operation == Operation::Initialize {
            event.adapter_initialized = Some(self.initialized);
        }
        if event.operation == Operation::Scan && event.terminal {
            self.scan_control.finish(event.operation_id);
        }
        if let Ok(device) = DeviceId::parse(&event.context.device_address) {
            event.connection_state = self.devices.get(&device).map(|session| session.state);
        }
        tokio::select! {
            result = self.event_tx.send(event) => { let _ = result; }
            () = self.shutdown.cancelled() => {}
        }
    }
}

async fn execute_device_action(
    backend: &dyn BleBackend,
    device: &DeviceId,
    action: &DeviceAction,
) -> Result<EventData, BleError> {
    match action {
        DeviceAction::Connect => backend.connect(device).await.map(|()| EventData::None),
        DeviceAction::Disconnect => backend.disconnect(device).await.map(|()| EventData::None),
        DeviceAction::DiscoverServices => backend
            .discover_services(device)
            .await
            .map(EventData::Services),
        DeviceAction::Read(key) => backend.read(device, key).await.map(EventData::Bytes),
        DeviceAction::Write {
            key,
            data,
            with_response,
        } => backend
            .write(device, key, data, *with_response)
            .await
            .map(|()| EventData::None),
        DeviceAction::Subscribe(key) => backend
            .subscribe(device, key)
            .await
            .map(|()| EventData::None),
        DeviceAction::Unsubscribe(key) => backend
            .unsubscribe(device, key)
            .await
            .map(|()| EventData::None),
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod regression_tests;
