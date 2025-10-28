use godot::prelude::*;

/// 特征值属性
#[derive(Clone, Debug)]
pub struct CharacteristicProperties {
    pub read: bool,
    pub write: bool,
    pub write_without_response: bool,
    pub notify: bool,
    pub indicate: bool,
}

impl CharacteristicProperties {
    /// 转换为 Godot Dictionary
    pub fn to_dictionary(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("read", self.read);
        dict.set("write", self.write);
        dict.set("write_without_response", self.write_without_response);
        dict.set("notify", self.notify);
        dict.set("indicate", self.indicate);
        dict
    }
}

/// BLE 特征值信息
#[derive(Clone, Debug)]
pub struct BleCharacteristicInfo {
    pub uuid: String,
    pub properties: CharacteristicProperties,
}

impl BleCharacteristicInfo {
    /// 创建新的特征值信息
    pub fn new(uuid: String, properties: CharacteristicProperties) -> Self {
        Self { uuid, properties }
    }

    /// 转换为 Godot Dictionary
    pub fn to_dictionary(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("uuid", self.uuid.clone());
        dict.set("properties", self.properties.to_dictionary());
        dict
    }
}
