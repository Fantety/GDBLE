# 设计文档

## 概述

GDBLE 插件采用分层架构设计，将 btleplug 的 Rust API 封装为 Godot 友好的节点类。设计遵循以下原则：

- **单一职责**：每个 Rust 文件包含一个主要的 struct，负责特定功能
- **异步处理**：使用 Tokio 运行时处理 btleplug 的异步操作
- **信号驱动**：通过 Godot 信号系统向 GDScript 传递事件和数据
- **线程安全**：使用 Arc 和 Mutex 确保跨线程访问的安全性
- **资源管理**：正确实现 Godot 的生命周期方法，确保资源清理

## 架构

### 整体架构图

```
┌─────────────────────────────────────────┐
│         GDScript Layer                  │
│  (Godot Scene, Game Logic)              │
└─────────────────┬───────────────────────┘
                  │ Godot Signals & Method Calls
┌─────────────────▼───────────────────────┐
│      GDExtension Layer (Rust)           │
│  ┌─────────────────────────────────┐    │
│  │  BluetoothManager (Node)        │    │
│  │  - 管理适配器和运行时           │    │
│  └──────────┬──────────────────────┘    │
│             │                            │
│  ┌──────────▼──────────┐  ┌──────────┐  │
│  │  BluetoothScanner   │  │ BleDevice│  │
│  │  - 设备扫描         │  │ - 设备连接│  │
│  └─────────────────────┘  │ - 服务发现│  │
│                            │ - 数据读写│  │
│                            └──────────┘  │
└─────────────────┬───────────────────────┘
                  │ btleplug API
┌─────────────────▼───────────────────────┐
│         btleplug Library                │
│  (Platform-specific BLE Stack)          │
└─────────────────────────────────────────┘
```

### 模块划分

项目将包含以下 Rust 模块文件：

1. **lib.rs** - 库入口和扩展注册
2. **bluetooth_manager.rs** - 蓝牙管理器节点
3. **bluetooth_scanner.rs** - 设备扫描功能
4. **ble_device.rs** - BLE 设备封装
5. **ble_characteristic.rs** - 特征值封装
6. **ble_service.rs** - 服务封装
7. **runtime.rs** - Tokio 运行时管理
8. **types.rs** - 共享类型定义和转换

## 组件和接口

### 1. BluetoothManager (bluetooth_manager.rs)

**职责**：
- 管理蓝牙适配器的初始化
- 管理 Tokio 异步运行时
- 作为 GDScript 的主要入口点
- 协调扫描器和设备对象

**GDScript 接口**：

```gdscript
class_name BluetoothManager extends Node

# 方法
func initialize() -> bool
func is_initialized() -> bool
func get_adapter_info() -> Dictionary
func start_scan(timeout_seconds: float = 10.0) -> void
func stop_scan() -> void
func get_discovered_devices() -> Array[Dictionary]
func connect_device(address: String) -> BleDevice
func disconnect_device(address: String) -> void

# 信号
signal adapter_initialized(success: bool, error: String)
signal device_discovered(device_info: Dictionary)
signal device_updated(device_info: Dictionary)
signal scan_started()
signal scan_stopped()
signal error_occurred(error_message: String)
```

**内部结构**：

```rust
#[derive(GodotClass)]
#[class(base=Node)]
pub struct BluetoothManager {
    base: Base<Node>,
    adapter: Option<Arc<Adapter>>,
    runtime: Option<Arc<Runtime>>,
    scanner: Option<Gd<BluetoothScanner>>,
    devices: HashMap<String, Gd<BleDevice>>,
}
```

### 2. BluetoothScanner (bluetooth_scanner.rs)

**职责**：
- 执行 BLE 设备扫描
- 管理扫描状态和超时
- 收集和更新设备信息

**内部接口**（不直接暴露给 GDScript）：

```rust
pub struct BluetoothScanner {
    adapter: Arc<Adapter>,
    runtime: Arc<Runtime>,
    is_scanning: Arc<Mutex<bool>>,
    discovered_devices: Arc<Mutex<HashMap<String, DeviceInfo>>>,
}

impl BluetoothScanner {
    pub fn new(adapter: Arc<Adapter>, runtime: Arc<Runtime>) -> Self;
    pub async fn start_scan(&self, timeout: Duration) -> Result<(), BleError>;
    pub fn stop_scan(&self);
    pub fn get_devices(&self) -> Vec<DeviceInfo>;
}
```

### 3. BleDevice (ble_device.rs)

**职责**：
- 表示单个 BLE 设备
- 管理设备连接状态
- 执行服务发现
- 提供特征值读写接口

**GDScript 接口**：

```gdscript
class_name BleDevice extends RefCounted

# 方法
func connect_async() -> void
func disconnect() -> void
func is_connected() -> bool
func get_address() -> String
func get_name() -> String
func discover_services() -> void
func get_services() -> Array[Dictionary]
func read_characteristic(service_uuid: String, char_uuid: String) -> void
func write_characteristic(service_uuid: String, char_uuid: String, data: PackedByteArray, with_response: bool = true) -> void
func subscribe_characteristic(service_uuid: String, char_uuid: String) -> void
func unsubscribe_characteristic(service_uuid: String, char_uuid: String) -> void

# 信号
signal connected()
signal disconnected()
signal connection_failed(error: String)
signal services_discovered(services: Array)
signal characteristic_read(char_uuid: String, data: PackedByteArray)
signal characteristic_written(char_uuid: String)
signal characteristic_notified(char_uuid: String, data: PackedByteArray)
signal operation_failed(operation: String, error: String)
```

**内部结构**：

```rust
#[derive(GodotClass)]
#[class(base=RefCounted)]
pub struct BleDevice {
    base: Base<RefCounted>,
    peripheral: Arc<Peripheral>,
    runtime: Arc<Runtime>,
    address: GString,
    name: GString,
    is_connected: Arc<Mutex<bool>>,
    services: Arc<Mutex<Vec<BleServiceInfo>>>,
}
```

### 4. BleService (ble_service.rs)

**职责**：
- 表示 BLE 服务的信息
- 存储服务 UUID 和特征值列表

**数据结构**：

```rust
#[derive(Clone)]
pub struct BleServiceInfo {
    pub uuid: String,
    pub characteristics: Vec<BleCharacteristicInfo>,
}

// 转换为 Godot Dictionary
impl BleServiceInfo {
    pub fn to_dictionary(&self) -> Dictionary;
}
```

### 5. BleCharacteristic (ble_characteristic.rs)

**职责**：
- 表示 BLE 特征值的信息
- 存储特征值属性（可读、可写、可通知等）

**数据结构**：

```rust
#[derive(Clone)]
pub struct BleCharacteristicInfo {
    pub uuid: String,
    pub properties: CharacteristicProperties,
}

#[derive(Clone)]
pub struct CharacteristicProperties {
    pub read: bool,
    pub write: bool,
    pub write_without_response: bool,
    pub notify: bool,
    pub indicate: bool,
}

impl BleCharacteristicInfo {
    pub fn to_dictionary(&self) -> Dictionary;
}
```

### 6. Runtime (runtime.rs)

**职责**：
- 管理 Tokio 异步运行时
- 提供统一的任务执行接口
- 处理运行时生命周期

**接口**：

```rust
pub struct RuntimeManager {
    runtime: Arc<Runtime>,
}

impl RuntimeManager {
    pub fn new() -> Self;
    pub fn runtime(&self) -> Arc<Runtime>;
    pub fn spawn<F>(&self, future: F) where F: Future + Send + 'static;
    pub fn block_on<F>(&self, future: F) -> F::Output where F: Future;
}
```

### 7. Types (types.rs)

**职责**：
- 定义共享的类型和枚举
- 提供 Rust 类型与 Godot 类型之间的转换
- 定义错误类型

**类型定义**：

```rust
// 设备信息
#[derive(Clone, Debug)]
pub struct DeviceInfo {
    pub address: String,
    pub name: Option<String>,
    pub rssi: Option<i16>,
}

impl DeviceInfo {
    pub fn to_dictionary(&self) -> Dictionary;
}

// 错误类型
#[derive(Debug)]
pub enum BleError {
    AdapterNotFound,
    DeviceNotFound,
    ConnectionFailed(String),
    OperationFailed(String),
    NotConnected,
    InvalidUuid(String),
}

impl BleError {
    pub fn to_string(&self) -> String;
}
```

## 数据模型

### 设备信息流

```
扫描阶段:
btleplug::Peripheral -> DeviceInfo -> Dictionary -> GDScript

连接阶段:
GDScript (address) -> BleDevice -> btleplug::Peripheral

服务发现:
btleplug::Service -> BleServiceInfo -> Dictionary -> GDScript

特征值操作:
GDScript (UUID + data) -> BleDevice -> btleplug::Characteristic
btleplug::Characteristic (notification) -> signal -> GDScript
```

### 数据转换规则

| Rust 类型 | Godot 类型 | 说明 |
|-----------|-----------|------|
| String | GString | 文本数据 |
| Vec<u8> | PackedByteArray | 字节数据 |
| DeviceInfo | Dictionary | 设备信息 |
| BleServiceInfo | Dictionary | 服务信息 |
| BleCharacteristicInfo | Dictionary | 特征值信息 |
| Option<T> | Variant | 可选值 |
| Result<T, E> | 通过信号传递 | 异步结果 |

## 错误处理

### 错误处理策略

1. **同步方法**：返回布尔值或空值，通过 `godot_error!` 宏输出错误日志
2. **异步操作**：通过信号传递错误信息
3. **致命错误**：在初始化阶段失败时，设置对象为无效状态

### 错误信号模式

所有异步操作都遵循以下信号模式：

```gdscript
# 成功信号
signal operation_completed(result: Variant)

# 失败信号
signal operation_failed(error_message: String)
```

### 错误日志级别

- **Error**：操作失败，影响功能
- **Warning**：非预期情况，但可以继续
- **Info**：重要操作的状态信息（仅调试模式）

## 测试策略

### 单元测试

每个模块应包含基本的单元测试：

1. **类型转换测试**：验证 Rust 和 Godot 类型之间的转换
2. **错误处理测试**：验证错误情况的正确处理
3. **状态管理测试**：验证对象状态的正确转换

### 集成测试

使用 Godot 场景进行集成测试：

1. **适配器初始化测试**：验证适配器的正确初始化
2. **设备扫描测试**：验证设备发现功能
3. **连接测试**：验证设备连接和断开
4. **数据传输测试**：验证特征值读写和通知

### 测试场景

创建 GDScript 测试场景：

```gdscript
extends Node

var bluetooth_manager: BluetoothManager

func _ready():
    bluetooth_manager = BluetoothManager.new()
    add_child(bluetooth_manager)
    
    bluetooth_manager.adapter_initialized.connect(_on_adapter_initialized)
    bluetooth_manager.device_discovered.connect(_on_device_discovered)
    
    bluetooth_manager.initialize()

func _on_adapter_initialized(success: bool, error: String):
    if success:
        print("Adapter initialized successfully")
        bluetooth_manager.start_scan(5.0)
    else:
        print("Failed to initialize: ", error)

func _on_device_discovered(device_info: Dictionary):
    print("Found device: ", device_info.name, " at ", device_info.address)
```

## 线程和并发

### 线程模型

```
Main Thread (Godot)
    │
    ├─> BluetoothManager (Node)
    │       │
    │       └─> 调用异步方法
    │
Tokio Runtime Thread Pool
    │
    ├─> 扫描任务
    ├─> 连接任务
    ├─> 读写任务
    └─> 通知监听任务
            │
            └─> 通过 callable_mp 回调主线程
                    │
                    └─> 发射 Godot 信号
```

### 线程安全机制

1. **Arc<Mutex<T>>**：用于跨线程共享可变状态
2. **Arc<T>**：用于跨线程共享不可变数据
3. **callable_mp**：用于从 Tokio 线程回调 Godot 主线程

### 异步操作模式

```rust
// 在 Godot 方法中启动异步任务
#[func]
fn connect_device(&mut self, address: GString) {
    let runtime = self.runtime.clone();
    let peripheral = self.peripheral.clone();
    let callable = self.base().callable("_on_connected");
    
    runtime.spawn(async move {
        match peripheral.connect().await {
            Ok(_) => {
                // 回调主线程发射信号
                callable.callv(array![true, GString::new()]);
            }
            Err(e) => {
                callable.callv(array![false, GString::from(e.to_string())]);
            }
        }
    });
}

// 主线程回调方法
#[func]
fn _on_connected(&mut self, success: bool, error: GString) {
    if success {
        self.base_mut().emit_signal("connected".into(), &[]);
    } else {
        self.base_mut().emit_signal("connection_failed".into(), &[error.to_variant()]);
    }
}
```

## 性能考虑

### 优化策略

1. **避免频繁的类型转换**：缓存转换后的 Dictionary 对象
2. **批量处理设备更新**：在扫描时合并多个设备更新
3. **使用对象池**：复用 BleDevice 对象
4. **延迟服务发现**：仅在需要时才发现服务

### 内存管理

1. **及时释放连接**：在设备断开时释放 Peripheral 对象
2. **清理订阅**：在取消订阅时清理通知处理器
3. **限制缓存大小**：限制已发现设备的缓存数量

## 平台特定考虑

### Windows
- 使用 WinRT API
- 需要应用清单声明蓝牙权限

### Linux
- 使用 BlueZ D-Bus API
- 需要用户在 bluetooth 组中

### macOS
- 使用 CoreBluetooth
- 需要在 Info.plist 中声明权限

### 平台抽象

btleplug 已经处理了平台差异，插件层面不需要额外的平台特定代码。但需要在文档中说明各平台的权限要求。

## 部署和构建

### 构建配置

```toml
[package]
name = "gdble"
version = "0.1.0"
edition = "2021"

[dependencies]
godot = "0.4.2"
btleplug = "0.11.8"
tokio = { version = "1.40", features = ["full"] }

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = 3
lto = true
```

### 输出结构

```
addons/gdble/
├── bin/
│   ├── windows/
│   │   └── gdble.dll
│   ├── linux/
│   │   └── libgdble.so
│   └── macos/
│       └── libgdble.dylib
└── gdble.gdextension
```

### GDExtension 配置

```ini
[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.2

[libraries]
windows.x86_64 = "res://addons/gdble/bin/windows/gdble.dll"
linux.x86_64 = "res://addons/gdble/bin/linux/libgdble.so"
macos = "res://addons/gdble/bin/macos/libgdble.dylib"
```
