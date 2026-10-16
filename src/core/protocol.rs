use std::collections::HashMap;
use std::time::Duration;

use uuid::Uuid;

use crate::ble_service::BleServiceInfo;
use crate::types::{AdapterInfo, BleError, DeviceInfo};

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
        let mut details = HashMap::new();
        match &error {
            BleError::AdapterUnavailable(state) => {
                details.insert("state".into(), state.clone());
            }
            BleError::EventOverflow(source) => {
                details.insert("source".into(), source.clone());
            }
            _ => {}
        }
        Self {
            code: error.code().to_string(),
            message: error.to_string(),
            retryable: error.is_retryable(),
            details,
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
    /// Internal authoritative snapshot; local request rejections leave it absent.
    pub connection_state: Option<ConnectionState>,
    pub adapter_initialized: Option<bool>,
    /// Internal delivery epoch; never serialized to GDScript.
    pub notification_generation: Option<u64>,
    pub discovery_generation: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
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
            connection_state: None,
            adapter_initialized: None,
            notification_generation: None,
            discovery_generation: None,
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
            connection_state: None,
            adapter_initialized: None,
            notification_generation: None,
            discovery_generation: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScanGeneration(u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ActiveScan {
    pub(super) generation: ScanGeneration,
    pub(super) operation_id: OperationId,
    pub(super) timeout: Option<Duration>,
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

    pub(super) fn active(&self) -> Option<ActiveScan> {
        self.active
    }
}
