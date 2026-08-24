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

/// 调试日志宏 - 仅在调试模式下输出
/// 使用 eprintln! 确保线程安全，可以从任何线程调用
#[macro_export]
macro_rules! ble_debug {
    ($($arg:tt)*) => {
        if $crate::types::is_debug_mode() {
            eprintln!("[BLE Debug] {}", format!($($arg)*));
        }
    };
}

/// 信息日志宏 - 仅在调试模式下输出
/// 使用 eprintln! 确保线程安全，可以从任何线程调用
#[macro_export]
macro_rules! ble_info {
    ($($arg:tt)*) => {
        if $crate::types::is_debug_mode() {
            eprintln!("[BLE Info] {}", format!($($arg)*));
        }
    };
}

/// 警告日志宏 - 仅在调试模式下输出
/// 使用 eprintln! 确保线程安全，可以从任何线程调用
#[macro_export]
macro_rules! ble_warn {
    ($($arg:tt)*) => {
        if $crate::types::is_debug_mode() {
            eprintln!("[BLE Warning] {}", format!($($arg)*));
        }
    };
}

/// 错误日志宏 - 仅在调试模式下输出
/// 使用 eprintln! 确保线程安全，可以从任何线程调用
#[macro_export]
macro_rules! ble_error {
    ($($arg:tt)*) => {
        if $crate::types::is_debug_mode() {
            eprintln!("[BLE Error] {}", format!($($arg)*));
        }
    };
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
#[allow(dead_code)]
#[derive(Debug, Clone, Eq, PartialEq, thiserror::Error)]
pub enum BleError {
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
    #[error("Operation is busy: {0}")]
    Busy(String),
    #[error("The command queue is full")]
    QueueFull,
    #[error("Operation cancelled: {0}")]
    Cancelled(String),
    #[error("Android Bluetooth bridge is not initialized: {0}")]
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
    #[error("Service not found: {0}")]
    ServiceNotFound(String),
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
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("Operation timed out: {0}")]
    Timeout(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl BleError {
    /// Preserves the 0.5.x Chinese signal text while structured events use Display.
    pub fn legacy_message(&self) -> String {
        match self {
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
            BleError::ServiceNotFound(uuid) => {
                format!("未找到服务 UUID: {}", uuid)
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
            BleError::PermissionDenied(msg) => {
                format!("权限被拒绝: {}", msg)
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
            BleError::ServiceNotFound(_) => "SERVICE_NOT_FOUND",
            BleError::CharacteristicNotFound(_) => "CHARACTERISTIC_NOT_FOUND",
            BleError::ScanFailed(_) => "SCAN_FAILED",
            BleError::InitializationFailed(_) => "INITIALIZATION_FAILED",
            BleError::ReadFailed(_) => "READ_FAILED",
            BleError::WriteFailed(_) => "WRITE_FAILED",
            BleError::SubscribeFailed(_) => "SUBSCRIBE_FAILED",
            BleError::UnsubscribeFailed(_) => "UNSUBSCRIBE_FAILED",
            BleError::ServiceDiscoveryFailed(_) => "SERVICE_DISCOVERY_FAILED",
            BleError::PermissionDenied(_) => "PERMISSION_DENIED",
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
