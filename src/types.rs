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
#[macro_export]
macro_rules! ble_debug {
    ($($arg:tt)*) => {
        if $crate::types::is_debug_mode() {
            godot::prelude::godot_print!("[BLE Debug] {}", format!($($arg)*));
        }
    };
}

/// 信息日志宏 - 仅在调试模式下输出
#[macro_export]
macro_rules! ble_info {
    ($($arg:tt)*) => {
        if $crate::types::is_debug_mode() {
            godot::prelude::godot_print!("[BLE Info] {}", format!($($arg)*));
        }
    };
}

/// 警告日志宏 - 仅在调试模式下输出
#[macro_export]
macro_rules! ble_warn {
    ($($arg:tt)*) => {
        if $crate::types::is_debug_mode() {
            godot::prelude::godot_warn!("[BLE Warning] {}", format!($($arg)*));
        }
    };
}

/// 错误日志宏 - 仅在调试模式下输出
#[macro_export]
macro_rules! ble_error {
    ($($arg:tt)*) => {
        if $crate::types::is_debug_mode() {
            godot::prelude::godot_error!("[BLE Error] {}", format!($($arg)*));
        }
    };
}

/// 设备信息结构
#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub address: String,
    pub name: Option<String>,
    pub rssi: Option<i16>,
}

impl DeviceInfo {
    /// 创建新的设备信息
    pub fn new(address: String, name: Option<String>, rssi: Option<i16>) -> Self {
        Self {
            address,
            name,
            rssi,
        }
    }

    /// 转换为 Godot Dictionary
    pub fn to_dictionary(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("address", self.address.clone());
        
        if let Some(ref name) = self.name {
            dict.set("name", name.clone());
        } else {
            dict.set("name", Variant::nil());
        }
        
        if let Some(rssi) = self.rssi {
            dict.set("rssi", rssi);
        } else {
            dict.set("rssi", Variant::nil());
        }
        
        dict
    }
}

/// BLE 错误类型
#[derive(Debug, Clone)]
pub enum BleError {
    /// 未找到蓝牙适配器
    AdapterNotFound,
    /// 未找到设备
    DeviceNotFound(String),
    /// 连接失败
    ConnectionFailed(String),
    /// 操作失败
    OperationFailed(String),
    /// 设备未连接
    NotConnected,
    /// 无效的 UUID
    InvalidUuid(String),
    /// 服务未找到
    ServiceNotFound(String),
    /// 特征值未找到
    CharacteristicNotFound(String),
    /// 扫描失败
    ScanFailed(String),
    /// 初始化失败
    InitializationFailed(String),
    /// 读取失败
    ReadFailed(String),
    /// 写入失败
    WriteFailed(String),
    /// 订阅失败
    SubscribeFailed(String),
    /// 取消订阅失败
    UnsubscribeFailed(String),
    /// 服务发现失败
    ServiceDiscoveryFailed(String),
    /// 权限错误
    PermissionDenied(String),
    /// 超时错误
    Timeout(String),
    /// 内部错误
    InternalError(String),
}

impl BleError {
    /// 转换为 GString (用于 Godot 信号)
    pub fn to_gstring(&self) -> GString {
        GString::from(self.to_string().as_str())
    }

    /// 转换为字符串描述
    pub fn to_string(&self) -> String {
        match self {
            BleError::AdapterNotFound => {
                "未找到蓝牙适配器，请确保系统蓝牙已启用".to_string()
            }
            BleError::DeviceNotFound(addr) => {
                format!("未找到指定的蓝牙设备: {}", addr)
            }
            BleError::ConnectionFailed(msg) => {
                format!("连接失败: {}", msg)
            }
            BleError::OperationFailed(msg) => {
                format!("操作失败: {}", msg)
            }
            BleError::NotConnected => {
                "设备未连接，请先连接设备".to_string()
            }
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

    /// 获取错误代码
    pub fn error_code(&self) -> &str {
        match self {
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

    /// 判断错误是否可重试
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            BleError::Timeout(_) | BleError::ConnectionFailed(_) | BleError::OperationFailed(_)
        )
    }

    /// 记录错误到 Godot 控制台
    pub fn log_error(&self) {
        godot_error!("[BLE Error] {}: {}", self.error_code(), self.to_string());
    }

    /// 记录警告到 Godot 控制台
    pub fn log_warning(&self) {
        godot_warn!("[BLE Warning] {}: {}", self.error_code(), self.to_string());
    }
}

impl std::fmt::Display for BleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_string())
    }
}

impl std::error::Error for BleError {}

/// 适配器信息结构
#[derive(Clone, Debug)]
pub struct AdapterInfo {
    pub name: String,
    pub address: Option<String>,
}

impl AdapterInfo {
    /// 创建新的适配器信息
    pub fn new(name: String, address: Option<String>) -> Self {
        Self { name, address }
    }

    /// 转换为 Godot Dictionary
    pub fn to_dictionary(&self) -> Dictionary {
        let mut dict = Dictionary::new();
        dict.set("name", self.name.clone());
        
        if let Some(ref address) = self.address {
            dict.set("address", address.clone());
        } else {
            dict.set("address", Variant::nil());
        }
        
        dict
    }
}

// Re-export BLE service and characteristic types from their modules
// These are used by other modules but not directly in this file
#[allow(unused_imports)]
pub use crate::ble_characteristic::{BleCharacteristicInfo, CharacteristicProperties};
#[allow(unused_imports)]
pub use crate::ble_service::BleServiceInfo;
