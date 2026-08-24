use std::collections::HashMap;
use std::time::Duration;

use godot::classes::notify::NodeNotification;
use godot::prelude::*;

use crate::ble_device::BleDevice;
use crate::core::{
    AdapterInfo, BleError, BleEvent, CoreClient, CoreRuntime, DeviceId, DeviceProgressKind,
    EventContext, EventData, Operation, OperationId, Phase,
};
use crate::godot_event;
use crate::types::{is_debug_mode, set_debug_mode, DeviceInfo};

const EVENTS_PER_FRAME: usize = 256;

/// Godot-facing adapter. All system BLE work is owned by the `BleCore` worker.
#[derive(GodotClass)]
#[class(base=Node)]
pub struct BluetoothManager {
    base: Base<Node>,
    core: Option<CoreRuntime>,
    client: CoreClient,
    initialized: bool,
    adapter_info: Option<AdapterInfo>,
    active_scan_id: OperationId,
    scan_started: bool,
    discovered_devices: HashMap<String, DeviceInfo>,
    devices: HashMap<String, Gd<BleDevice>>,
}

#[godot_api]
impl INode for BluetoothManager {
    fn init(base: Base<Node>) -> Self {
        let core = CoreRuntime::spawn_production();
        let client = core.client();
        Self {
            base,
            core: Some(core),
            client,
            initialized: false,
            adapter_info: None,
            active_scan_id: OperationId::UNSOLICITED,
            scan_started: false,
            discovered_devices: HashMap::new(),
            devices: HashMap::new(),
        }
    }

    fn ready(&mut self) {
        self.base_mut().set_process(true);
    }

    fn process(&mut self, _delta: f64) {
        for _ in 0..EVENTS_PER_FRAME {
            let Some(event) = self.core.as_mut().and_then(CoreRuntime::try_recv) else {
                break;
            };
            self.apply_core_event(event);
        }
    }

    fn on_notification(&mut self, what: NodeNotification) {
        if what == NodeNotification::PREDELETE {
            if let Some(mut core) = self.core.take() {
                core.shutdown();
            }
            self.devices.clear();
            self.initialized = false;
        }
    }
}

#[godot_api]
impl BluetoothManager {
    #[signal]
    fn ble_event(event: VarDictionary);

    #[signal]
    fn adapter_initialized(success: bool, error: GString);

    #[signal]
    fn device_discovered(device_info: VarDictionary);

    #[signal]
    fn device_updated(device_info: VarDictionary);

    #[signal]
    fn scan_started();

    #[signal]
    fn scan_stopped();

    #[signal]
    fn error_occurred(error_message: GString);

    #[signal]
    fn device_connecting(address: GString);

    #[signal]
    fn device_connected(address: GString);

    #[signal]
    fn device_disconnected(address: GString);

    #[func]
    pub fn set_debug_mode(&self, enabled: bool) {
        set_debug_mode(enabled);
    }

    #[func]
    pub fn is_debug_mode(&self) -> bool {
        is_debug_mode()
    }

    #[func]
    pub fn initialize(&self) -> i64 {
        self.client.initialize().get()
    }

    #[func]
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    #[func]
    pub fn get_adapter_info(&self) -> VarDictionary {
        self.adapter_info
            .as_ref()
            .map_or_else(VarDictionary::new, AdapterInfo::to_dictionary)
    }

    #[func]
    pub fn start_scan(&mut self, timeout_seconds: f64) -> i64 {
        let timeout = match Self::scan_timeout(timeout_seconds) {
            Ok(timeout) => timeout,
            Err(error) => {
                return self
                    .client
                    .reject(Operation::Scan, EventContext::default(), error)
                    .get();
            }
        };
        let operation_id = self.client.start_scan(timeout);
        if self.active_scan_id == OperationId::UNSOLICITED {
            self.active_scan_id = operation_id;
            self.scan_started = false;
        }
        operation_id.get()
    }

    #[func]
    pub fn stop_scan(&self) -> i64 {
        self.client.stop_scan(self.active_scan_id).get()
    }

    #[func]
    pub fn get_discovered_devices(&self) -> Array<VarDictionary> {
        self.discovered_devices
            .values()
            .map(DeviceInfo::to_dictionary)
            .collect()
    }

    #[func]
    pub fn get_or_create_device(&mut self, address: GString) -> Option<Gd<BleDevice>> {
        let input = address.to_string();
        let device_id = match DeviceId::parse(&input) {
            Ok(device_id) => device_id,
            Err(error) => {
                self.client
                    .reject(Operation::ResolveDevice, EventContext::default(), error);
                return None;
            }
        };
        if !self.initialized {
            self.client.reject(
                Operation::ResolveDevice,
                EventContext::for_device(&device_id),
                BleError::NotInitialized,
            );
            return None;
        }
        let key = device_id.as_str().to_string();
        if let Some(device) = self.devices.get(&key) {
            return Some(device.clone());
        }

        let known = self.discovered_devices.get(&key);
        let display_address =
            known.map_or_else(|| input.trim().to_string(), |info| info.address.clone());
        let name = known.and_then(|info| info.name.clone()).unwrap_or_default();
        let device = BleDevice::new(key.clone(), display_address, name, self.client.clone());
        self.devices.insert(key, device.clone());
        Some(device)
    }

    #[func]
    pub fn connect_device(&mut self, address: GString) -> Option<Gd<BleDevice>> {
        self.get_or_create_device(address)
    }

    #[func]
    pub fn disconnect_device(&self, address: GString) -> i64 {
        let input = address.to_string();
        let device_id = match DeviceId::parse(&input) {
            Ok(device_id) => device_id,
            Err(error) => {
                return self
                    .client
                    .reject(Operation::Disconnect, EventContext::default(), error)
                    .get();
            }
        };
        let context = EventContext::for_device(&device_id);
        if !self.devices.contains_key(device_id.as_str()) {
            return self
                .client
                .reject(
                    Operation::Disconnect,
                    context,
                    BleError::DeviceNotFound(input.trim().to_string()),
                )
                .get();
        }
        self.client.disconnect(device_id.as_str()).get()
    }

    #[func]
    pub fn get_device(&self, address: GString) -> Option<Gd<BleDevice>> {
        let device_id = DeviceId::parse(&address.to_string()).ok()?;
        self.devices.get(device_id.as_str()).cloned()
    }

    #[func]
    pub fn get_connected_devices(&self) -> Array<Gd<BleDevice>> {
        self.devices
            .values()
            .filter(|device| device.bind().is_connected())
            .cloned()
            .collect()
    }
}

impl BluetoothManager {
    fn scan_timeout(timeout_seconds: f64) -> Result<Option<Duration>, BleError> {
        if !timeout_seconds.is_finite() {
            return Err(BleError::InvalidArgument(
                "scan timeout must be a finite number".to_string(),
            ));
        }
        if timeout_seconds < 0.0 {
            return Err(BleError::InvalidArgument(
                "scan timeout must be greater than or equal to 0".to_string(),
            ));
        }
        if timeout_seconds == 0.0 {
            Ok(None)
        } else {
            Ok(Some(Duration::from_secs_f64(timeout_seconds)))
        }
    }

    fn apply_core_event(&mut self, event: BleEvent) {
        self.update_caches(&event);

        if !event.context.device_address.is_empty() {
            if let Some(mut device) = self.devices.get(&event.context.device_address).cloned() {
                device.bind_mut().apply_core_event(&event);
            }
        }

        self.base_mut().emit_signal(
            "ble_event",
            &[godot_event::to_dictionary(&event).to_variant()],
        );
        self.emit_legacy_manager_signals(&event);
    }

    fn update_caches(&mut self, event: &BleEvent) {
        match (&event.operation, &event.phase, &event.data) {
            (Operation::Initialize, Phase::Succeeded, EventData::Adapter(info)) => {
                self.initialized = true;
                self.adapter_info = Some(info.clone());
            }
            (Operation::Initialize, Phase::Failed | Phase::Cancelled, _) => {
                self.initialized = false;
                self.adapter_info = None;
            }
            (Operation::Scan, Phase::Progress, EventData::Device { info, .. }) => {
                if let Ok(device_id) = DeviceId::parse(&info.address) {
                    self.discovered_devices
                        .insert(device_id.as_str().to_string(), info.clone());
                }
            }
            _ => {}
        }
    }

    fn emit_legacy_manager_signals(&mut self, event: &BleEvent) {
        match (event.operation, event.phase) {
            (Operation::Initialize, Phase::Succeeded) => {
                self.base_mut().emit_signal(
                    "adapter_initialized",
                    &[true.to_variant(), GString::new().to_variant()],
                );
            }
            (Operation::Initialize, Phase::Failed | Phase::Cancelled) => {
                let error = legacy_error(event);
                self.base_mut().emit_signal(
                    "adapter_initialized",
                    &[false.to_variant(), error.to_variant()],
                );
                self.base_mut()
                    .emit_signal("error_occurred", &[error.to_variant()]);
            }
            (Operation::Scan, Phase::Started) => {
                if event.operation_id == self.active_scan_id && !self.scan_started {
                    self.scan_started = true;
                    self.base_mut().emit_signal("scan_started", &[]);
                }
            }
            (Operation::Scan, Phase::Progress) => {
                if let EventData::Device { kind, info } = &event.data {
                    let signal = match kind {
                        DeviceProgressKind::Discovered => "device_discovered",
                        DeviceProgressKind::Updated => "device_updated",
                    };
                    self.base_mut()
                        .emit_signal(signal, &[info.to_dictionary().to_variant()]);
                }
            }
            (Operation::Scan, Phase::Succeeded | Phase::Failed | Phase::Cancelled) => {
                if matches!(event.phase, Phase::Failed | Phase::Cancelled) {
                    let error = legacy_error(event);
                    self.base_mut()
                        .emit_signal("error_occurred", &[error.to_variant()]);
                }
                if event.operation_id == self.active_scan_id {
                    self.active_scan_id = OperationId::UNSOLICITED;
                    if self.scan_started {
                        self.base_mut().emit_signal("scan_stopped", &[]);
                    }
                    self.scan_started = false;
                }
            }
            (Operation::Connect, Phase::Started) => {
                self.base_mut().emit_signal(
                    "device_connecting",
                    &[GString::from(event.context.device_address.as_str()).to_variant()],
                );
            }
            (Operation::Connect, Phase::Succeeded) => {
                self.base_mut().emit_signal(
                    "device_connected",
                    &[GString::from(event.context.device_address.as_str()).to_variant()],
                );
            }
            (Operation::Disconnect, Phase::Succeeded | Phase::Received) => {
                self.base_mut().emit_signal(
                    "device_disconnected",
                    &[GString::from(event.context.device_address.as_str()).to_variant()],
                );
            }
            (Operation::ResolveDevice, Phase::Failed | Phase::Cancelled) => {
                let error = legacy_error(event);
                self.base_mut()
                    .emit_signal("error_occurred", &[error.to_variant()]);
            }
            _ => {}
        }
    }
}

fn legacy_error(event: &BleEvent) -> GString {
    GString::from(
        event
            .error
            .as_ref()
            .map_or("操作失败", |error| error.legacy_message.as_str()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_timeout_accepts_continuous_and_timed_scans() {
        assert_eq!(BluetoothManager::scan_timeout(0.0).unwrap(), None);
        assert_eq!(
            BluetoothManager::scan_timeout(2.5).unwrap(),
            Some(Duration::from_secs_f64(2.5))
        );
    }

    #[test]
    fn scan_timeout_rejects_negative_and_non_finite_values() {
        for invalid in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let error = BluetoothManager::scan_timeout(invalid).unwrap_err();
            assert_eq!(error.code(), "INVALID_ARGUMENT");
        }
    }

    #[test]
    fn per_frame_event_budget_is_bounded() {
        assert_eq!(EVENTS_PER_FRAME, 256);
    }
}
