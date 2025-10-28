use godot::prelude::*;
use crate::ble_characteristic::BleCharacteristicInfo;

/// BLE 服务信息
#[derive(Clone, Debug)]
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

    /// 转换为 Godot Dictionary
    pub fn to_dictionary(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("uuid", self.uuid.clone());
        
        let chars_array: Array<Dictionary> = self.characteristics
            .iter()
            .map(|char_info| char_info.to_dictionary())
            .collect();
        
        dict.set("characteristics", chars_array);
        
        dict
    }
}
