use godot::prelude::*;

use crate::ble_service::BleServiceInfo;
use crate::core::{
    BleError, BleEvent, CoreClient, EventContext, EventData, Operation, OperationId, Phase,
};
use crate::godot_event;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeviceState {
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

/// Main-thread Godot adapter for one normalized BLE device identity.
#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct BleDevice {
    base: Base<RefCounted>,
    client: Option<CoreClient>,
    address: String,
    display_address: GString,
    name: GString,
    state: DeviceState,
    services: Vec<BleServiceInfo>,
    next_local_operation_id: i64,
}

#[godot_api]
impl BleDevice {
    #[signal]
    fn ble_event(event: VarDictionary);

    #[signal]
    fn connected();

    #[signal]
    fn disconnected();

    #[signal]
    fn connection_failed(error: GString);

    #[signal]
    fn services_discovered(services: Array<VarDictionary>);

    #[signal]
    fn characteristic_read(char_uuid: GString, data: PackedByteArray);

    #[signal]
    fn characteristic_written(char_uuid: GString);

    #[signal]
    fn characteristic_notified(char_uuid: GString, data: PackedByteArray);

    #[signal]
    fn operation_failed(operation: GString, error: GString);

    #[func]
    fn connect_async(&mut self) -> i64 {
        if let Some(client) = self.client.clone() {
            client.connect(&self.address).get()
        } else {
            self.reject_local(Operation::Connect, EventContext::default())
        }
    }

    #[func]
    fn disconnect(&mut self) -> i64 {
        if let Some(client) = self.client.clone() {
            client.disconnect(&self.address).get()
        } else {
            self.reject_local(Operation::Disconnect, EventContext::default())
        }
    }

    #[func]
    pub fn is_connected(&self) -> bool {
        self.state == DeviceState::Connected
    }

    #[func]
    fn get_address(&self) -> GString {
        self.display_address.clone()
    }

    #[func]
    fn get_name(&self) -> GString {
        self.name.clone()
    }

    #[func]
    fn discover_services(&mut self) -> i64 {
        if let Some(client) = self.client.clone() {
            client.discover_services(&self.address).get()
        } else {
            self.reject_local(Operation::DiscoverServices, EventContext::default())
        }
    }

    #[func]
    fn get_services(&self) -> Array<VarDictionary> {
        self.services
            .iter()
            .map(BleServiceInfo::to_dictionary)
            .collect()
    }

    #[func]
    fn read_characteristic(&mut self, service_uuid: GString, char_uuid: GString) -> i64 {
        if let Some(client) = self.client.clone() {
            client
                .read(
                    &self.address,
                    &service_uuid.to_string(),
                    &char_uuid.to_string(),
                )
                .get()
        } else {
            self.reject_local(Operation::Read, gatt_context(&service_uuid, &char_uuid))
        }
    }

    #[func]
    fn write_characteristic(
        &mut self,
        service_uuid: GString,
        char_uuid: GString,
        data: PackedByteArray,
        with_response: bool,
    ) -> i64 {
        if let Some(client) = self.client.clone() {
            client
                .write(
                    &self.address,
                    &service_uuid.to_string(),
                    &char_uuid.to_string(),
                    data.to_vec(),
                    with_response,
                )
                .get()
        } else {
            self.reject_local(Operation::Write, gatt_context(&service_uuid, &char_uuid))
        }
    }

    #[func]
    fn subscribe_characteristic(&mut self, service_uuid: GString, char_uuid: GString) -> i64 {
        if let Some(client) = self.client.clone() {
            client
                .subscribe(
                    &self.address,
                    &service_uuid.to_string(),
                    &char_uuid.to_string(),
                )
                .get()
        } else {
            self.reject_local(
                Operation::Subscribe,
                gatt_context(&service_uuid, &char_uuid),
            )
        }
    }

    #[func]
    fn unsubscribe_characteristic(&mut self, service_uuid: GString, char_uuid: GString) -> i64 {
        if let Some(client) = self.client.clone() {
            client
                .unsubscribe(
                    &self.address,
                    &service_uuid.to_string(),
                    &char_uuid.to_string(),
                )
                .get()
        } else {
            self.reject_local(
                Operation::Unsubscribe,
                gatt_context(&service_uuid, &char_uuid),
            )
        }
    }
}

#[godot_api]
impl IRefCounted for BleDevice {
    fn init(base: Base<RefCounted>) -> Self {
        Self {
            base,
            client: None,
            address: String::new(),
            display_address: GString::new(),
            name: GString::new(),
            state: DeviceState::Disconnected,
            services: Vec::new(),
            next_local_operation_id: 1,
        }
    }
}

impl BleDevice {
    pub(crate) fn new(
        address: String,
        display_address: String,
        name: String,
        client: CoreClient,
    ) -> Gd<Self> {
        Gd::from_init_fn(|base| Self {
            base,
            client: Some(client),
            address,
            display_address: GString::from(display_address.as_str()),
            name: GString::from(name.as_str()),
            state: DeviceState::Disconnected,
            services: Vec::new(),
            next_local_operation_id: 1,
        })
    }

    pub(crate) fn apply_core_event(&mut self, event: &BleEvent) {
        if event.context.device_address != self.address {
            return;
        }

        if let EventData::Device { info, .. } = &event.data {
            self.display_address = GString::from(info.address.as_str());
            if let Some(name) = &info.name {
                self.name = GString::from(name.as_str());
            }
        }

        self.base_mut().emit_signal(
            "ble_event",
            &[godot_event::to_dictionary(event).to_variant()],
        );

        match (event.operation, event.phase) {
            (Operation::Connect, Phase::Started) => self.state = DeviceState::Connecting,
            (Operation::Connect, Phase::Succeeded) => {
                self.state = DeviceState::Connected;
                self.base_mut().emit_signal("connected", &[]);
            }
            (Operation::Connect, Phase::Failed | Phase::Cancelled) => {
                self.state = DeviceState::Disconnected;
                let error = legacy_error(event);
                self.base_mut()
                    .emit_signal("connection_failed", &[error.to_variant()]);
            }
            (Operation::Disconnect, Phase::Started) => self.state = DeviceState::Disconnecting,
            (Operation::Disconnect, Phase::Succeeded | Phase::Received) => {
                self.state = DeviceState::Disconnected;
                self.base_mut().emit_signal("disconnected", &[]);
            }
            (Operation::Disconnect, Phase::Failed | Phase::Cancelled) => {
                self.state = DeviceState::Connected;
                self.emit_operation_failed(event);
            }
            (Operation::DiscoverServices, Phase::Succeeded) => {
                if let EventData::Services(services) = &event.data {
                    self.services = services.clone();
                    let dictionaries: Array<VarDictionary> =
                        services.iter().map(BleServiceInfo::to_dictionary).collect();
                    self.base_mut()
                        .emit_signal("services_discovered", &[dictionaries.to_variant()]);
                }
            }
            (Operation::Read, Phase::Succeeded) => {
                if let EventData::Bytes(data) = &event.data {
                    let data = PackedByteArray::from(data.as_slice());
                    self.base_mut().emit_signal(
                        "characteristic_read",
                        &[
                            GString::from(event.context.characteristic_uuid.as_str()).to_variant(),
                            data.to_variant(),
                        ],
                    );
                }
            }
            (Operation::Write, Phase::Succeeded) => {
                self.base_mut().emit_signal(
                    "characteristic_written",
                    &[GString::from(event.context.characteristic_uuid.as_str()).to_variant()],
                );
            }
            (Operation::Notification, Phase::Received) => {
                if let EventData::Bytes(data) = &event.data {
                    let data = PackedByteArray::from(data.as_slice());
                    self.base_mut().emit_signal(
                        "characteristic_notified",
                        &[
                            GString::from(event.context.characteristic_uuid.as_str()).to_variant(),
                            data.to_variant(),
                        ],
                    );
                }
            }
            (
                Operation::DiscoverServices
                | Operation::Read
                | Operation::Write
                | Operation::Subscribe
                | Operation::Unsubscribe,
                Phase::Failed | Phase::Cancelled,
            ) => self.emit_operation_failed(event),
            _ => {}
        }
    }

    fn emit_operation_failed(&mut self, event: &BleEvent) {
        let operation = match event.operation {
            Operation::DiscoverServices => "discover_services",
            Operation::Read => "read_characteristic",
            Operation::Write => "write_characteristic",
            Operation::Subscribe => "subscribe_characteristic",
            Operation::Unsubscribe => "unsubscribe_characteristic",
            Operation::Disconnect => "disconnect",
            _ => event.operation.as_str(),
        };
        self.base_mut().emit_signal(
            "operation_failed",
            &[
                GString::from(operation).to_variant(),
                legacy_error(event).to_variant(),
            ],
        );
    }

    fn reject_local(&mut self, operation: Operation, context: EventContext) -> i64 {
        let operation_id = OperationId::new(self.next_local_operation_id);
        self.next_local_operation_id = self.next_local_operation_id.saturating_add(1);
        let event = BleEvent::failed(operation, operation_id, context, BleError::NotInitialized);
        self.apply_core_event(&event);
        operation_id.get()
    }
}

fn gatt_context(service_uuid: &GString, characteristic_uuid: &GString) -> EventContext {
    EventContext {
        device_address: String::new(),
        service_uuid: service_uuid.to_string(),
        characteristic_uuid: characteristic_uuid.to_string(),
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
