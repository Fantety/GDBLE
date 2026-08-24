use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;
use tokio::task::{AbortHandle, JoinSet};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::backend::{
    BackendEvent, BackendEventStream, BackendNotification, BleBackend, BtleplugBackend,
};
use crate::ble_service::BleServiceInfo;
pub use crate::types::{AdapterInfo, BleError, DeviceInfo};

pub const COMMAND_CHANNEL_CAPACITY: usize = 64;
pub const EVENT_CHANNEL_CAPACITY: usize = 1024;
const INTERNAL_CHANNEL_CAPACITY: usize = 256;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct DeviceId(String);

impl DeviceId {
    pub fn parse(value: &str) -> Result<Self, BleError> {
        let value = value.trim();
        if value.is_empty() {
            return Err(BleError::InvalidArgument(
                "device address is empty".to_string(),
            ));
        }
        Ok(Self(value.to_ascii_lowercase()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct GattKey {
    service_uuid: String,
    characteristic_uuid: String,
}

impl GattKey {
    pub fn parse(service_uuid: &str, characteristic_uuid: &str) -> Result<Self, BleError> {
        Ok(Self {
            service_uuid: canonical_uuid(service_uuid)?,
            characteristic_uuid: canonical_uuid(characteristic_uuid)?,
        })
    }

    pub fn service_uuid(&self) -> &str {
        &self.service_uuid
    }

    pub fn characteristic_uuid(&self) -> &str {
        &self.characteristic_uuid
    }
}

fn canonical_uuid(value: &str) -> Result<String, BleError> {
    let value = value.trim();
    Uuid::parse_str(value)
        .map(|uuid| uuid.hyphenated().to_string())
        .map_err(|_| BleError::InvalidUuid(value.to_string()))
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OperationId(i64);

impl OperationId {
    pub const UNSOLICITED: Self = Self(0);

    pub const fn new(value: i64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    Initialize,
    Scan,
    Connect,
    Disconnect,
    DiscoverServices,
    Read,
    Write,
    Subscribe,
    Unsubscribe,
    Notification,
    ResolveDevice,
}

impl Operation {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Initialize => "initialize",
            Self::Scan => "scan",
            Self::Connect => "connect",
            Self::Disconnect => "disconnect",
            Self::DiscoverServices => "discover_services",
            Self::Read => "read",
            Self::Write => "write",
            Self::Subscribe => "subscribe",
            Self::Unsubscribe => "unsubscribe",
            Self::Notification => "notification",
            Self::ResolveDevice => "resolve_device",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Started,
    Progress,
    Succeeded,
    Failed,
    Cancelled,
    Received,
}

impl Phase {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Progress => "progress",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Received => "received",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EventContext {
    pub device_address: String,
    pub service_uuid: String,
    pub characteristic_uuid: String,
}

impl EventContext {
    pub fn for_device(device: &DeviceId) -> Self {
        Self {
            device_address: device.as_str().to_string(),
            ..Self::default()
        }
    }

    pub fn for_gatt(device: &DeviceId, key: &GattKey) -> Self {
        Self {
            device_address: device.as_str().to_string(),
            service_uuid: key.service_uuid.clone(),
            characteristic_uuid: key.characteristic_uuid.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceProgressKind {
    Discovered,
    Updated,
}

impl DeviceProgressKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Discovered => "discovered",
            Self::Updated => "updated",
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum EventData {
    #[default]
    None,
    Adapter(AdapterInfo),
    Device {
        kind: DeviceProgressKind,
        info: DeviceInfo,
    },
    Services(Vec<BleServiceInfo>),
    Bytes(Vec<u8>),
    Reason(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub details: HashMap<String, String>,
    pub legacy_message: String,
}

impl From<BleError> for EventError {
    fn from(error: BleError) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.to_string(),
            retryable: error.is_retryable(),
            details: HashMap::new(),
            legacy_message: error.legacy_message(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BleEvent {
    pub operation: Operation,
    pub phase: Phase,
    pub operation_id: OperationId,
    pub terminal: bool,
    pub context: EventContext,
    pub data: EventData,
    pub error: Option<EventError>,
}

impl BleEvent {
    pub fn started(operation: Operation, operation_id: OperationId, context: EventContext) -> Self {
        Self::new(operation, Phase::Started, operation_id, false, context)
    }

    pub fn progress(operation_id: OperationId, context: EventContext, data: EventData) -> Self {
        Self {
            operation: Operation::Scan,
            phase: Phase::Progress,
            operation_id,
            terminal: false,
            context,
            data,
            error: None,
        }
    }

    pub fn succeeded(
        operation: Operation,
        operation_id: OperationId,
        context: EventContext,
        data: EventData,
    ) -> Self {
        let mut event = Self::new(operation, Phase::Succeeded, operation_id, true, context);
        event.data = data;
        event
    }

    pub fn failed(
        operation: Operation,
        operation_id: OperationId,
        context: EventContext,
        error: BleError,
    ) -> Self {
        let phase = if matches!(error, BleError::Cancelled(_)) {
            Phase::Cancelled
        } else {
            Phase::Failed
        };
        let mut event = Self::new(operation, phase, operation_id, true, context);
        event.error = Some(error.into());
        event
    }

    pub fn received(operation: Operation, context: EventContext, data: EventData) -> Self {
        let mut event = Self::new(
            operation,
            Phase::Received,
            OperationId::UNSOLICITED,
            false,
            context,
        );
        event.data = data;
        event
    }

    fn new(
        operation: Operation,
        phase: Phase,
        operation_id: OperationId,
        terminal: bool,
        context: EventContext,
    ) -> Self {
        Self {
            operation,
            phase,
            operation_id,
            terminal,
            context,
            data: EventData::None,
            error: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScanGeneration(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ActiveScan {
    generation: ScanGeneration,
    operation_id: OperationId,
    timeout: Option<Duration>,
}

#[derive(Debug, Default)]
pub struct ScanSession {
    generation: u64,
    active: Option<ActiveScan>,
}

impl ScanSession {
    pub fn start(
        &mut self,
        operation_id: OperationId,
        timeout: Option<Duration>,
    ) -> Result<ScanGeneration, BleError> {
        if self.active.is_some() {
            return Err(BleError::Busy("scan already active".to_string()));
        }
        self.generation = self.generation.wrapping_add(1);
        let generation = ScanGeneration(self.generation);
        self.active = Some(ActiveScan {
            generation,
            operation_id,
            timeout,
        });
        Ok(generation)
    }

    pub fn finish(&mut self, generation: ScanGeneration) -> Result<bool, BleError> {
        if self.active.map(|active| active.generation) != Some(generation) {
            return Ok(false);
        }
        self.active = None;
        Ok(true)
    }

    pub fn active_generation(&self) -> Option<ScanGeneration> {
        self.active.map(|active| active.generation)
    }

    fn active(&self) -> Option<ActiveScan> {
        self.active
    }
}

#[derive(Clone)]
pub struct CoreClient {
    command_tx: mpsc::Sender<CoreCommand>,
    event_tx: mpsc::Sender<BleEvent>,
    next_operation_id: Arc<AtomicI64>,
}

impl CoreClient {
    pub fn initialize(&self) -> OperationId {
        self.submit(
            Operation::Initialize,
            EventContext::default(),
            |operation_id| CoreCommand::Initialize { operation_id },
        )
    }

    pub fn start_scan(&self, timeout: Option<Duration>) -> OperationId {
        self.submit(Operation::Scan, EventContext::default(), |operation_id| {
            CoreCommand::StartScan {
                operation_id,
                timeout,
            }
        })
    }

    pub fn stop_scan(&self, operation_id: OperationId) -> OperationId {
        if operation_id != OperationId::UNSOLICITED {
            let _ = self
                .command_tx
                .try_send(CoreCommand::StopScan { operation_id });
        }
        operation_id
    }

    pub fn connect(&self, address: &str) -> OperationId {
        self.device_command(Operation::Connect, address, |operation_id, device| {
            CoreCommand::Device {
                operation_id,
                device,
                action: DeviceAction::Connect,
            }
        })
    }

    pub fn disconnect(&self, address: &str) -> OperationId {
        self.device_command(Operation::Disconnect, address, |operation_id, device| {
            CoreCommand::Device {
                operation_id,
                device,
                action: DeviceAction::Disconnect,
            }
        })
    }

    pub fn discover_services(&self, address: &str) -> OperationId {
        self.device_command(
            Operation::DiscoverServices,
            address,
            |operation_id, device| CoreCommand::Device {
                operation_id,
                device,
                action: DeviceAction::DiscoverServices,
            },
        )
    }

    pub fn read(
        &self,
        address: &str,
        service_uuid: &str,
        characteristic_uuid: &str,
    ) -> OperationId {
        self.gatt_command(
            Operation::Read,
            address,
            service_uuid,
            characteristic_uuid,
            |operation_id, device, key| CoreCommand::Device {
                operation_id,
                device,
                action: DeviceAction::Read(key),
            },
        )
    }

    pub fn write(
        &self,
        address: &str,
        service_uuid: &str,
        characteristic_uuid: &str,
        data: Vec<u8>,
        with_response: bool,
    ) -> OperationId {
        self.gatt_command(
            Operation::Write,
            address,
            service_uuid,
            characteristic_uuid,
            move |operation_id, device, key| CoreCommand::Device {
                operation_id,
                device,
                action: DeviceAction::Write {
                    key,
                    data,
                    with_response,
                },
            },
        )
    }

    pub fn reject(
        &self,
        operation: Operation,
        context: EventContext,
        error: BleError,
    ) -> OperationId {
        let operation_id = self.next_id();
        self.emit_local_failure(operation, operation_id, context, error);
        operation_id
    }

    pub fn subscribe(
        &self,
        address: &str,
        service_uuid: &str,
        characteristic_uuid: &str,
    ) -> OperationId {
        self.gatt_command(
            Operation::Subscribe,
            address,
            service_uuid,
            characteristic_uuid,
            |operation_id, device, key| CoreCommand::Device {
                operation_id,
                device,
                action: DeviceAction::Subscribe(key),
            },
        )
    }

    pub fn unsubscribe(
        &self,
        address: &str,
        service_uuid: &str,
        characteristic_uuid: &str,
    ) -> OperationId {
        self.gatt_command(
            Operation::Unsubscribe,
            address,
            service_uuid,
            characteristic_uuid,
            |operation_id, device, key| CoreCommand::Device {
                operation_id,
                device,
                action: DeviceAction::Unsubscribe(key),
            },
        )
    }

    fn next_id(&self) -> OperationId {
        OperationId::new(
            self.next_operation_id
                .fetch_add(1, Ordering::Relaxed)
                .max(1),
        )
    }

    fn submit(
        &self,
        operation: Operation,
        context: EventContext,
        command: impl FnOnce(OperationId) -> CoreCommand,
    ) -> OperationId {
        let operation_id = self.next_id();
        if self.command_tx.try_send(command(operation_id)).is_err() {
            self.emit_local_failure(operation, operation_id, context, BleError::QueueFull);
        }
        operation_id
    }

    fn device_command(
        &self,
        operation: Operation,
        address: &str,
        command: impl FnOnce(OperationId, DeviceId) -> CoreCommand,
    ) -> OperationId {
        let operation_id = self.next_id();
        match DeviceId::parse(address) {
            Ok(device) => {
                let context = EventContext::for_device(&device);
                if self
                    .command_tx
                    .try_send(command(operation_id, device))
                    .is_err()
                {
                    self.emit_local_failure(operation, operation_id, context, BleError::QueueFull);
                }
            }
            Err(error) => {
                self.emit_local_failure(operation, operation_id, EventContext::default(), error)
            }
        }
        operation_id
    }

    fn gatt_command(
        &self,
        operation: Operation,
        address: &str,
        service_uuid: &str,
        characteristic_uuid: &str,
        command: impl FnOnce(OperationId, DeviceId, GattKey) -> CoreCommand,
    ) -> OperationId {
        let operation_id = self.next_id();
        let parsed = DeviceId::parse(address).and_then(|device| {
            GattKey::parse(service_uuid, characteristic_uuid).map(|key| (device, key))
        });
        match parsed {
            Ok((device, key)) => {
                let context = EventContext::for_gatt(&device, &key);
                if self
                    .command_tx
                    .try_send(command(operation_id, device, key))
                    .is_err()
                {
                    self.emit_local_failure(operation, operation_id, context, BleError::QueueFull);
                }
            }
            Err(error) => {
                self.emit_local_failure(operation, operation_id, EventContext::default(), error)
            }
        }
        operation_id
    }

    fn emit_local_failure(
        &self,
        operation: Operation,
        operation_id: OperationId,
        context: EventContext,
        error: BleError,
    ) {
        let _ = self
            .event_tx
            .try_send(BleEvent::failed(operation, operation_id, context, error));
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
        let (command_tx, command_rx) = mpsc::channel(COMMAND_CHANNEL_CAPACITY);
        let (event_tx, event_rx) = mpsc::channel(EVENT_CHANNEL_CAPACITY);
        let client = CoreClient {
            command_tx,
            event_tx: event_tx.clone(),
            next_operation_id: Arc::new(AtomicI64::new(1)),
        };
        let backend: Arc<dyn BleBackend> = backend;
        let shutdown = CancellationToken::new();
        let worker_shutdown = shutdown.clone();
        let worker = std::thread::Builder::new()
            .name("gdble-core".to_string())
            .spawn(move || {
                let runtime = match tokio::runtime::Builder::new_multi_thread()
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
                    run_core(backend, command_rx, event_tx, worker_shutdown).await;
                    let _ = done_tx.send(());
                });
                let _ = done_rx.recv();
                drop(core_task);
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
            let _ = self.client.command_tx.try_send(CoreCommand::Shutdown);
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

enum CoreCommand {
    Initialize {
        operation_id: OperationId,
    },
    StartScan {
        operation_id: OperationId,
        timeout: Option<Duration>,
    },
    StopScan {
        operation_id: OperationId,
    },
    Device {
        operation_id: OperationId,
        device: DeviceId,
        action: DeviceAction,
    },
    Shutdown,
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
    Backend(Result<BackendEvent, BleError>),
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
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
}

async fn run_core(
    backend: Arc<dyn BleBackend>,
    mut command_rx: mpsc::Receiver<CoreCommand>,
    event_tx: mpsc::Sender<BleEvent>,
    shutdown: CancellationToken,
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
                    Some(CoreCommand::Shutdown) | None => {
                        state.shutdown(&mut tasks).await;
                        break;
                    }
                    Some(command) => state.handle_command(command, &mut tasks).await,
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
            } => {
                self.start_scan(operation_id, timeout, tasks).await;
            }
            CoreCommand::StopScan { operation_id } => {
                self.stop_scan(operation_id, "stopped", tasks).await;
            }
            CoreCommand::Device {
                operation_id,
                device,
                action,
            } => {
                self.device_command(operation_id, device, action, tasks)
                    .await;
            }
            CoreCommand::Shutdown => {}
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
        tasks.spawn(async move {
            let result = backend.initialize().await;
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
        self.scan.stop_requested = false;
        self.scan.cancellation = Some(CancellationToken::new());
        let backend = self.backend.clone();
        let internal_tx = self.internal_tx.clone();
        tasks.spawn(async move {
            let result = backend.start_scan().await;
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
        tasks.spawn(async move {
            let result = backend.stop_scan().await;
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
            let abort_handle = tasks.spawn(async move {
                let result =
                    execute_device_action(backend.as_ref(), &device_for_task, &action_for_task)
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
                existing.characteristic_uuid == key.characteristic_uuid
                    && existing.service_uuid != key.service_uuid
            }) {
                return Some(Err(BleError::AmbiguousCharacteristic(
                    key.characteristic_uuid.clone(),
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
            InternalEvent::Backend(result) => self.backend_event(result, tasks).await,
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
                notification,
            } => self.notification(device, notification).await,
            InternalEvent::NotificationPumpFailed {
                device,
                generation,
                error,
            } => {
                self.finish_notification_pump(&device, generation);
                self.emit(BleEvent {
                    operation: Operation::Notification,
                    phase: Phase::Failed,
                    operation_id: OperationId::UNSOLICITED,
                    terminal: false,
                    context: EventContext::for_device(&device),
                    data: EventData::None,
                    error: Some(error.into()),
                })
                .await;
            }
            InternalEvent::NotificationPumpEnded { device, generation } => {
                self.finish_notification_pump(&device, generation);
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
                let internal_tx = self.internal_tx.clone();
                tasks.spawn(async move {
                    while let Some(event) = stream.next().await {
                        if internal_tx
                            .send(InternalEvent::Backend(event))
                            .await
                            .is_err()
                        {
                            break;
                        }
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
                if self.scan.stop_requested {
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
        if session.notification_cancellation.is_some() {
            return;
        }
        let cancellation = CancellationToken::new();
        session.notification_cancellation = Some(cancellation.clone());
        session.notification_generation = session.notification_generation.wrapping_add(1);
        let generation = session.notification_generation;
        let backend = self.backend.clone();
        let internal_tx = self.internal_tx.clone();
        let device = device.clone();
        tasks.spawn(async move {
            let mut stream = match backend.notifications(&device).await {
                Ok(stream) => stream,
                Err(error) => {
                    let _ = internal_tx.send(InternalEvent::NotificationPumpFailed { device, generation, error }).await;
                    return;
                }
            };
            loop {
                tokio::select! {
                    () = cancellation.cancelled() => break,
                    notification = stream.next() => match notification {
                        Some(Ok(notification)) => {
                            if internal_tx.send(InternalEvent::Notification { device: device.clone(), notification }).await.is_err() { break; }
                        }
                        Some(Err(error)) => {
                            let _ = internal_tx.send(InternalEvent::NotificationPumpFailed { device: device.clone(), generation, error }).await;
                            return;
                        }
                        None => break,
                    }
                }
            }
            let _ = internal_tx.send(InternalEvent::NotificationPumpEnded { device, generation }).await;
        });
    }

    fn finish_notification_pump(&mut self, device: &DeviceId, generation: u64) {
        if let Some(session) = self.devices.get_mut(device) {
            if session.notification_generation == generation {
                session.notification_cancellation = None;
            }
        }
    }

    async fn notification(&mut self, device: DeviceId, notification: BackendNotification) {
        let Some(session) = self.devices.get(&device) else {
            return;
        };
        let characteristic_uuid = notification.characteristic_uuid.to_ascii_lowercase();
        let Some(key) = session
            .subscriptions
            .iter()
            .find(|key| key.characteristic_uuid == characteristic_uuid)
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

    async fn shutdown(&mut self, tasks: &mut JoinSet<()>) {
        if let Some(cancellation) = self.scan.cancellation.take() {
            cancellation.cancel();
        }
        if self.scan.tracker.active().is_some() {
            let _ = self.backend.stop_scan().await;
        }
        for (device, session) in &mut self.devices {
            if let Some(in_flight) = session.in_flight.take() {
                in_flight.abort_handle.abort();
            }
            if let Some(cancellation) = session.notification_cancellation.take() {
                cancellation.cancel();
            }
            session.pending.clear();
            if session.state == ConnectionState::Connected {
                let _ = self.backend.disconnect(device).await;
            }
        }
        tasks.abort_all();
        while tasks.join_next().await.is_some() {}
    }

    async fn emit(&self, event: BleEvent) {
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
mod tests {
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    use crate::backend::test_support::FakeBackend;

    use super::*;

    fn wait_for_event(
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
        let init_id = client.initialize();
        wait_for_event(&mut runtime, |event| {
            event.operation_id == init_id && event.phase == Phase::Succeeded
        });
        let failed_id = client.start_scan(None);
        let failed = wait_for_event(&mut runtime, |event| {
            event.operation_id == failed_id && event.terminal
        });
        assert_eq!(failed.phase, Phase::Failed);
        let retry_id = client.start_scan(None);
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
        let init_id = client.initialize();
        wait_for_event(&mut runtime, |event| {
            event.operation_id == init_id && event.phase == Phase::Succeeded
        });
        let active_id = client.start_scan(None);
        wait_for_event(&mut runtime, |event| {
            event.operation_id == active_id && event.phase == Phase::Started
        });
        let rejected_id = client.start_scan(None);
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
        let init_id = client.initialize();
        wait_for_event(&mut runtime, |event| {
            event.operation_id == init_id && event.phase == Phase::Succeeded
        });
        let scan_id = client.start_scan(Some(Duration::from_millis(10)));
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
        let init_id = client.initialize();
        wait_for_event(&mut runtime, |event| {
            event.operation_id == init_id && event.phase == Phase::Succeeded
        });
        backend.emit_device_discovered("AA:BB:CC:DD:EE:FF");
        wait_for_event(&mut runtime, |event| {
            event.operation == Operation::Scan && event.phase == Phase::Progress
        });
        let connect_id = client.connect("aa:bb:cc:dd:ee:ff");
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
        let init_id = client.initialize();
        wait_for_event(&mut runtime, |event| {
            event.operation_id == init_id && event.phase == Phase::Succeeded
        });

        backend.block_next_connect();
        let stale_id = client.connect("AA:BB:CC:DD:EE:FF");
        wait_for_event(&mut runtime, |event| {
            event.operation_id == stale_id && event.phase == Phase::Started
        });
        backend.emit_remote_disconnect("AA:BB:CC:DD:EE:FF");
        let cancelled = wait_for_event(&mut runtime, |event| {
            event.operation_id == stale_id && event.terminal
        });
        assert_eq!(cancelled.phase, Phase::Cancelled);

        let reconnect_id = client.connect("AA:BB:CC:DD:EE:FF");
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
        let init_id = client.initialize();
        wait_for_event(&mut runtime, |event| {
            event.operation_id == init_id && event.phase == Phase::Succeeded
        });

        let failed_id = client.connect("AA:BB:CC:DD:EE:FF");
        let failed = wait_for_event(&mut runtime, |event| {
            event.operation_id == failed_id && event.terminal
        });
        assert_eq!(failed.phase, Phase::Failed);

        let retry_id = client.connect("AA:BB:CC:DD:EE:FF");
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
        let init_id = client.initialize();
        wait_for_event(&mut runtime, |event| {
            event.operation_id == init_id && event.phase == Phase::Succeeded
        });
        let connect_id = client.connect("AA:BB:CC:DD:EE:FF");
        wait_for_event(&mut runtime, |event| {
            event.operation_id == connect_id && event.phase == Phase::Succeeded
        });
        let first_id = client.subscribe(
            "AA:BB:CC:DD:EE:FF",
            "0000180d-0000-1000-8000-00805f9b34fb",
            "00002a37-0000-1000-8000-00805f9b34fb",
        );
        wait_for_event(&mut runtime, |event| {
            event.operation_id == first_id && event.phase == Phase::Succeeded
        });
        let second_id = client.subscribe(
            "AA:BB:CC:DD:EE:FF",
            "0000180f-0000-1000-8000-00805f9b34fb",
            "00002a37-0000-1000-8000-00805f9b34fb",
        );
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
        let init_id = client.initialize();
        wait_for_event(&mut runtime, |event| {
            event.operation_id == init_id && event.phase == Phase::Succeeded
        });

        let connect_id = client.connect("AA:BB:CC:DD:EE:FF");
        let discover_id = client.discover_services("AA:BB:CC:DD:EE:FF");
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
        let init_id = client.initialize();
        wait_for_event(&mut runtime, |event| {
            event.operation_id == init_id && event.phase == Phase::Succeeded
        });
        let connect_id = client.connect("AA:BB:CC:DD:EE:FF");
        wait_for_event(&mut runtime, |event| {
            event.operation_id == connect_id && event.phase == Phase::Succeeded
        });

        let service = "0000180d-0000-1000-8000-00805f9b34fb";
        let characteristic = "00002a37-0000-1000-8000-00805f9b34fb";
        let with_response =
            client.write("AA:BB:CC:DD:EE:FF", service, characteristic, vec![1], true);
        let without_response =
            client.write("AA:BB:CC:DD:EE:FF", service, characteristic, vec![2], false);
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
        let init_id = client.initialize();
        wait_for_event(&mut runtime, |event| {
            event.operation_id == init_id && event.phase == Phase::Succeeded
        });
        let connect_id = client.connect("AA:BB:CC:DD:EE:FF");
        wait_for_event(&mut runtime, |event| {
            event.operation_id == connect_id && event.phase == Phase::Succeeded
        });

        let service_a = "0000180d-0000-1000-8000-00805f9b34fb";
        let service_b = "0000180f-0000-1000-8000-00805f9b34fb";
        let characteristic = "00002a37-0000-1000-8000-00805f9b34fb";
        let subscribe_id = client.subscribe("AA:BB:CC:DD:EE:FF", service_a, characteristic);
        wait_for_event(&mut runtime, |event| {
            event.operation_id == subscribe_id && event.phase == Phase::Succeeded
        });
        let duplicate_id = client.subscribe("AA:BB:CC:DD:EE:FF", service_a, characteristic);
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
        let restart_id = client.subscribe("AA:BB:CC:DD:EE:FF", service_a, characteristic);
        wait_for_event(&mut runtime, |event| {
            event.operation_id == restart_id && event.phase == Phase::Succeeded
        });
        let restart_deadline = Instant::now() + Duration::from_secs(1);
        while backend.notification_pumps_started() != 2 {
            assert!(Instant::now() < restart_deadline);
            thread::sleep(Duration::from_millis(5));
        }

        backend.fail_next_unsubscribe(BleError::UnsubscribeFailed("radio busy".to_string()));
        let unsubscribe_id = client.unsubscribe("AA:BB:CC:DD:EE:FF", service_a, characteristic);
        let unsubscribe = wait_for_event(&mut runtime, |event| {
            event.operation_id == unsubscribe_id && event.terminal
        });
        assert_eq!(unsubscribe.phase, Phase::Failed);

        let ambiguous_id = client.subscribe("AA:BB:CC:DD:EE:FF", service_b, characteristic);
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
        let reconnect_id = client.connect("AA:BB:CC:DD:EE:FF");
        wait_for_event(&mut runtime, |event| {
            event.operation_id == reconnect_id && event.phase == Phase::Succeeded
        });
        let after_disconnect_id = client.subscribe("AA:BB:CC:DD:EE:FF", service_b, characteristic);
        wait_for_event(&mut runtime, |event| {
            event.operation_id == after_disconnect_id && event.phase == Phase::Succeeded
        });
    }

    #[test]
    fn shutdown_completes_when_the_event_queue_is_full() {
        let backend = Arc::new(FakeBackend::default());
        let mut runtime = CoreRuntime::spawn_with_backend(backend);
        let client = runtime.client();
        for _ in 0..(EVENT_CHANNEL_CAPACITY * 2) {
            client.connect("   ");
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
        let init_id = client.initialize();
        wait_for_event(&mut runtime, |event| {
            event.operation_id == init_id && event.phase == Phase::Succeeded
        });

        backend.block_next_connect();
        let connect_id = client.connect("AA:BB:CC:DD:EE:FF");
        wait_for_event(&mut runtime, |event| {
            event.operation_id == connect_id && event.phase == Phase::Started
        });
        for _ in 0..(COMMAND_CHANNEL_CAPACITY * 2) {
            client.discover_services("AA:BB:CC:DD:EE:FF");
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
}
