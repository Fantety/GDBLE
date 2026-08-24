use godot::prelude::*;

use crate::core::{BleEvent, EventData};

pub fn to_dictionary(event: &BleEvent) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    let operation = StringName::from(event.operation.as_str());
    let phase = StringName::from(event.phase.as_str());
    dictionary.set("operation", &operation);
    dictionary.set("phase", &phase);
    dictionary.set("operation_id", event.operation_id.get());
    dictionary.set("terminal", event.terminal);
    dictionary.set("device_address", event.context.device_address.as_str());
    dictionary.set("service_uuid", event.context.service_uuid.as_str());
    dictionary.set(
        "characteristic_uuid",
        event.context.characteristic_uuid.as_str(),
    );
    dictionary.set("data", &event_data_to_variant(&event.data));
    dictionary.set("error", &event_error_to_dictionary(event));
    dictionary
}

fn event_data_to_variant(data: &EventData) -> Variant {
    match data {
        EventData::None => Variant::nil(),
        EventData::Adapter(info) => info.to_dictionary().to_variant(),
        EventData::Device { kind, info } => {
            let mut dictionary = info.to_dictionary();
            let kind = StringName::from(kind.as_str());
            dictionary.set("kind", &kind);
            dictionary.to_variant()
        }
        EventData::Services(services) => {
            let services: Array<VarDictionary> = services
                .iter()
                .map(|service| service.to_dictionary())
                .collect();
            services.to_variant()
        }
        EventData::Bytes(bytes) => PackedByteArray::from(bytes.as_slice()).to_variant(),
        EventData::Reason(reason) => GString::from(reason.as_str()).to_variant(),
    }
}

fn event_error_to_dictionary(event: &BleEvent) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    let mut details = VarDictionary::new();
    if let Some(error) = &event.error {
        for (key, value) in &error.details {
            details.set(key.as_str(), value.as_str());
        }
        dictionary.set("code", error.code.as_str());
        dictionary.set("message", error.message.as_str());
        dictionary.set("retryable", error.retryable);
    } else {
        dictionary.set("code", "");
        dictionary.set("message", "");
        dictionary.set("retryable", false);
    }
    dictionary.set("details", &details);
    dictionary
}
