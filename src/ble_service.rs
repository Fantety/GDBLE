use crate::ble_characteristic::BleCharacteristicInfo;
use godot::prelude::*;

/// BLE 服务信息
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BleServiceInfo {
    pub uuid: String,
    pub characteristics: Vec<BleCharacteristicInfo>,
}

impl BleServiceInfo {
    /// 创建新的服务信息
    pub fn new(uuid: String, characteristics: Vec<BleCharacteristicInfo>) -> Self {
        Self {
            uuid,
            characteristics,
        }
    }

    /// 转换为 Godot VarDictionary
    pub fn to_dictionary(&self) -> VarDictionary {
        let mut dict = VarDictionary::new();
        dict.set("uuid", self.uuid.clone());

        let chars_array: Array<VarDictionary> = self
            .characteristics
            .iter()
            .map(|char_info| char_info.to_dictionary())
            .collect();

        dict.set("characteristics", &chars_array);

        dict
    }
}
