use godot::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

/// 全局调试模式标志
static DEBUG_MODE: AtomicBool = AtomicBool::new(false);

/// 设置调试模式
pub fn set_debug_mode(enabled: bool) {
    DEBUG_MODE.store(enabled, Ordering::Relaxed);
}

/// 检查是否启用调试模式
pub fn is_debug_mode() -> bool {
    DEBUG_MODE.load(Ordering::Relaxed)
}

/// 设备信息结构
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceInfo {
    pub address: String,
    pub name: Option<String>,
    pub rssi: Option<i16>,
    pub services: Vec<String>,
    pub manufacturer_data: std::collections::HashMap<u16, Vec<u8>>,
    pub service_data: std::collections::HashMap<String, Vec<u8>>,
    pub tx_power_level: Option<i16>,
}

impl DeviceInfo {
    /// 创建新的设备信息
    pub fn new(
        address: String,
        name: Option<String>,
        rssi: Option<i16>,
        services: Vec<String>,
        manufacturer_data: std::collections::HashMap<u16, Vec<u8>>,
        service_data: std::collections::HashMap<String, Vec<u8>>,
        tx_power_level: Option<i16>,
    ) -> Self {
        Self {
            address,
            name,
            rssi,
            services,
            manufacturer_data,
            service_data,
            tx_power_level,
        }
    }

    /// 转换为 Godot VarDictionary
    pub fn to_dictionary(&self) -> VarDictionary {
        let mut dict = VarDictionary::new();
        dict.set("address", self.address.clone());

        if let Some(ref name) = self.name {
            dict.set("name", name.clone());
        } else {
            dict.set("name", &Variant::nil());
        }

        if let Some(rssi) = self.rssi {
            dict.set("rssi", rssi);
        } else {
            dict.set("rssi", &Variant::nil());
        }

        // Services
        let mut services_array: Array<GString> = Array::new();
        for service in &self.services {
            services_array.push(&GString::from(service));
        }
        dict.set("services", &services_array);

        // Manufacturer Data
        let mut manuf_dict = VarDictionary::new();
        for (id, data) in &self.manufacturer_data {
            let mut byte_array = PackedByteArray::new();
            for byte in data {
                byte_array.push(*byte);
            }
            manuf_dict.set(*id, &byte_array);
        }
        dict.set("manufacturer_data", &manuf_dict);

        // Service Data
        let mut service_data_dict = VarDictionary::new();
        for (uuid_str, data) in &self.service_data {
            let mut byte_array = PackedByteArray::new();
            for byte in data {
                byte_array.push(*byte);
            }
            service_data_dict.set(uuid_str.as_str(), &byte_array);
        }
        dict.set("service_data", &service_data_dict);

        // TX Power Level
        if let Some(tx) = self.tx_power_level {
            dict.set("tx_power_level", tx);
        } else {
            dict.set("tx_power_level", &Variant::nil());
        }

        dict
    }
}

/// Error values passed through the Rust core and exposed as stable event codes.
#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum BleError {
    #[error("Bluetooth adapter is unavailable: {0}")]
    AdapterUnavailable(String),
    #[error("Event buffer overflow: {0}")]
    EventOverflow(String),
    #[error("GATT services changed; discover services again")]
    ServicesChanged,
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Operation is busy: {0}")]
    Busy(String),
    #[error("The command queue is full")]
    QueueFull,
    #[error("Operation cancelled: {0}")]
    Cancelled(String),
    #[error("Android Bluetooth bridge is not initialized: {0}")]
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    AndroidNotInitialized(String),
    #[error("Characteristic UUID is ambiguous across services: {0}")]
    AmbiguousCharacteristic(String),
    #[error("Bluetooth is not initialized")]
    NotInitialized,
    #[error("Bluetooth adapter not found")]
    AdapterNotFound,
    #[error("Device not found: {0}")]
    DeviceNotFound(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    #[error("Device is not connected")]
    NotConnected,
    #[error("Invalid UUID: {0}")]
    InvalidUuid(String),
    #[error("Characteristic not found: {0}")]
    CharacteristicNotFound(String),
    #[error("Scan failed: {0}")]
    ScanFailed(String),
    #[error("Initialization failed: {0}")]
    InitializationFailed(String),
    #[error("Read failed: {0}")]
    ReadFailed(String),
    #[error("Write failed: {0}")]
    WriteFailed(String),
    #[error("Subscribe failed: {0}")]
    SubscribeFailed(String),
    #[error("Unsubscribe failed: {0}")]
    UnsubscribeFailed(String),
    #[error("Service discovery failed: {0}")]
    ServiceDiscoveryFailed(String),
    #[error("Operation timed out: {0}")]
    Timeout(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl BleError {
    /// Preserves the 0.5.x Chinese signal text while structured events use Display.
    pub fn legacy_message(&self) -> String {
        match self {
            BleError::AdapterUnavailable(state) => format!("蓝牙适配器不可用: {state}"),
            BleError::EventOverflow(source) => format!("事件缓冲溢出: {source}"),
            BleError::ServicesChanged => "GATT 服务已变化，请重新发现服务".to_string(),
            BleError::InvalidArgument(msg) => format!("参数无效: {}", msg),
            BleError::Busy(msg) => format!("操作忙: {}", msg),
            BleError::QueueFull => "操作队列已满，请稍后重试".to_string(),
            BleError::Cancelled(msg) => format!("操作已取消: {}", msg),
            BleError::AndroidNotInitialized(msg) => {
                format!("Android 蓝牙桥未初始化: {}", msg)
            }
            BleError::AmbiguousCharacteristic(uuid) => {
                format!("特征值 UUID 在多个服务中重复: {}", uuid)
            }
            BleError::NotInitialized => "蓝牙尚未初始化".to_string(),
            BleError::AdapterNotFound => "未找到蓝牙适配器，请确保系统蓝牙已启用".to_string(),
            BleError::DeviceNotFound(addr) => {
                format!("未找到指定的蓝牙设备: {}", addr)
            }
            BleError::ConnectionFailed(msg) => {
                format!("连接失败: {}", msg)
            }
            BleError::OperationFailed(msg) => {
                format!("操作失败: {}", msg)
            }
            BleError::NotConnected => "设备未连接，请先连接设备".to_string(),
            BleError::InvalidUuid(uuid) => {
                format!("无效的 UUID: {}", uuid)
            }
            BleError::CharacteristicNotFound(uuid) => {
                format!("未找到特征值 UUID: {}", uuid)
            }
            BleError::ScanFailed(msg) => {
                format!("扫描失败: {}", msg)
            }
            BleError::InitializationFailed(msg) => {
                format!("初始化失败: {}", msg)
            }
            BleError::ReadFailed(msg) => {
                format!("读取特征值失败: {}", msg)
            }
            BleError::WriteFailed(msg) => {
                format!("写入特征值失败: {}", msg)
            }
            BleError::SubscribeFailed(msg) => {
                format!("订阅通知失败: {}", msg)
            }
            BleError::UnsubscribeFailed(msg) => {
                format!("取消订阅失败: {}", msg)
            }
            BleError::ServiceDiscoveryFailed(msg) => {
                format!("服务发现失败: {}", msg)
            }
            BleError::Timeout(msg) => {
                format!("操作超时: {}", msg)
            }
            BleError::InternalError(msg) => {
                format!("内部错误: {}", msg)
            }
        }
    }

    /// Returns the stable machine-readable error code.
    pub fn error_code(&self) -> &str {
        match self {
            BleError::AdapterUnavailable(_) => "ADAPTER_UNAVAILABLE",
            BleError::EventOverflow(_) => "EVENT_OVERFLOW",
            BleError::ServicesChanged => "SERVICES_CHANGED",
            BleError::InvalidArgument(_) => "INVALID_ARGUMENT",
            BleError::Busy(_) => "BUSY",
            BleError::QueueFull => "QUEUE_FULL",
            BleError::Cancelled(_) => "CANCELLED",
            BleError::AndroidNotInitialized(_) => "ANDROID_NOT_INITIALIZED",
            BleError::AmbiguousCharacteristic(_) => "AMBIGUOUS_CHARACTERISTIC",
            BleError::NotInitialized => "NOT_INITIALIZED",
            BleError::AdapterNotFound => "ADAPTER_NOT_FOUND",
            BleError::DeviceNotFound(_) => "DEVICE_NOT_FOUND",
            BleError::ConnectionFailed(_) => "CONNECTION_FAILED",
            BleError::OperationFailed(_) => "OPERATION_FAILED",
            BleError::NotConnected => "NOT_CONNECTED",
            BleError::InvalidUuid(_) => "INVALID_UUID",
            BleError::CharacteristicNotFound(_) => "CHARACTERISTIC_NOT_FOUND",
            BleError::ScanFailed(_) => "SCAN_FAILED",
            BleError::InitializationFailed(_) => "INITIALIZATION_FAILED",
            BleError::ReadFailed(_) => "READ_FAILED",
            BleError::WriteFailed(_) => "WRITE_FAILED",
            BleError::SubscribeFailed(_) => "SUBSCRIBE_FAILED",
            BleError::UnsubscribeFailed(_) => "UNSUBSCRIBE_FAILED",
            BleError::ServiceDiscoveryFailed(_) => "SERVICE_DISCOVERY_FAILED",
            BleError::Timeout(_) => "TIMEOUT",
            BleError::InternalError(_) => "INTERNAL_ERROR",
        }
    }

    pub fn code(&self) -> &str {
        self.error_code()
    }

    /// 判断错误是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            BleError::Timeout(_)
                | BleError::ConnectionFailed(_)
                | BleError::OperationFailed(_)
                | BleError::QueueFull
                | BleError::AdapterUnavailable(_)
                | BleError::EventOverflow(_)
        )
    }
}

/// 适配器信息结构
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterInfo {
    pub name: String,
    pub address: Option<String>,
}

impl AdapterInfo {
    /// 创建新的适配器信息
    pub fn new(name: String, address: Option<String>) -> Self {
        Self { name, address }
    }

    /// 转换为 Godot VarDictionary
    pub fn to_dictionary(&self) -> VarDictionary {
        let mut dict = VarDictionary::new();
        dict.set("name", self.name.clone());

        if let Some(ref address) = self.address {
            dict.set("address", address.clone());
        } else {
            dict.set("address", &Variant::nil());
        }

        dict
    }
}
