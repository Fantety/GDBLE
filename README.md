<div align="center">
  <h1>GDBLE - Godot Bluetooth Low Energy Plugin</h1>
<p>
<img src="GodotBLEx229.png" alt="GDBLE Logo"/>
</p>
<p>
    <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/Fantety/GDBLE/release.yml">
    <img alt="GitHub code size in bytes" src="https://img.shields.io/github/languages/code-size/Fantety/GDBLE">
    <img alt="GitHub language count" src="https://img.shields.io/github/languages/count/Fantety/GDBLE">
    <img alt="GitHub License" src="https://img.shields.io/github/license/Fantety/GDBLE">
  </p>
  <p><i>一个为 Godot 4 开发的现代化低功耗蓝牙（BLE）插件</i></p>
  <p>
    <a href="README.md">🇨🇳 中文</a> |
    <a href="README_EN.md">🇺🇸 English</a>
  </p>
</div>

---

## 📖 目录

- [概述](#概述)
- [特性](#特性)
- [平台支持](#平台支持)
- [安装](#安装)
- [快速开始](#快速开始)
- [API 参考](#api-参考)
- [完整示例](#完整示例)
- [调试模式](#调试模式)
- [常见问题](#常见问题)
- [数据格式说明](#数据格式说明)
- [最佳实践](#最佳实践)
- [性能优化建议](#性能优化建议)
- [故障排除](#故障排除)
- [贡献](#贡献)
- [许可证](#许可证)
- [致谢](#致谢)
- [联系方式](#联系方式)

---

## 概述

**当前版本：0.6.0 · Godot 4.2+**

本轮修复的测试与待完成发行验收见 [修复验收记录](docs/REPAIR_VALIDATION.md)。发布前必须通过源码摘要、最终 addon 冒烟和真机验收；`BUILD_INFO.json` 的 `release_ready = false` 表示产物尚未齐备。

面向 Godot 4.2+ 的 Rust BLE GDExtension，基于 btleplug 0.12.0。支持扫描、连接、GATT 服务发现、读写和通知，并保留 0.5.x 的类名、方法参数与旧信号。

**插件地址**: [GDBLE - Godot Asset Library](https://godotengine.org/asset-library/asset/3439)

### 为什么选择 GDBLE？

- ✅ **现代化架构**: 使用 Rust 和异步运行时，性能优异
- ✅ **实时响应**: 非阻塞操作，后台执行 BLE 请求
- ✅ **完整功能**: 支持扫描、连接、读写、通知等所有 BLE 操作
- ✅ **易于使用**: 简洁的 GDScript API，信号驱动的事件系统
- ✅ **可调试**: 内置调试模式，方便开发和排错

---

## 特性

### 核心功能

- 🔍 **设备扫描**: 扫描附近的 BLE 设备，获取设备名称、地址和信号强度
- 🔗 **设备连接**: 连接到指定的 BLE 设备
- 🔎 **服务发现**: 发现设备的 GATT 服务和特征
- 📖 **特征读取**: 读取特征值
- ✍️ **特征写入**: 写入数据到特征（支持有响应和无响应模式）
- 🔔 **通知订阅**: 订阅特征通知，实时接收数据更新
- 🎯 **多设备管理**: 同时管理多个 BLE 设备连接

### 技术特性

- ⚡ **异步执行**: BLE 请求由后台 Core 处理，Godot 主线程读取缓存
- 🔒 **线程边界**: 后台不持有 Godot 对象，信号在主线程发出
- 📊 **统一事件**: operation ID、稳定错误码和完整 GATT 上下文
- 🐛 **故障诊断**: 结构化失败事件、操作超时和兼容调试接口
- 🎮 **兼容旧项目**: 保留 0.5.x 类名、方法参数和旧信号

---

## 平台支持

| 平台 | 架构 | 最低要求 |
| --- | --- | --- |
| ✅ Windows | x86_64 | Windows 10 |
| ✅ Linux | x86_64 | 发行版需提供 BlueZ/DBus |
| ✅ macOS | x86_64、ARM64 | macOS 10.15 / 11 |
| ✅ Android | ARM64、x86_64 | API 23，compileSdk 34 |

不支持 Android ARMv7、iOS 和自动重连。Android ARM64 用于真机，x86_64 主要用于模拟器加载测试。

### 线程安全说明

- 后台 BLE Core 统一持有运行时、设备会话和通知流。
- 命令和事件通过有界 Channel 传递，Manager 每帧最多处理 256 个排队事件。
- Godot 对象与信号只在主线程访问，缓存先更新再发出信号。

---

## 安装

### 方法 1: 从 Asset Library 安装（推荐）

1. 在 Godot 编辑器中打开 **AssetLib**
2. 搜索 "GDBLE" 或 "Bluetooth"
3. 点击下载并安装

### 方法 2: 手动安装

1. 从 [Releases](https://github.com/Fantety/GDBLE/releases) 下载发行包。
2. 将包内完整的 `addons/gdble` 复制到项目中的同名目录。
3. 重启 Godot 编辑器，确认 GDExtension 已加载。

发行目录结构如下；各平台使用对应动态库，Android 使用两个 AAR：

```text
your_project/
└── addons/
    └── gdble/
        ├── gdble.gdextension
        ├── gdble.gdap
        ├── permission_startup.gd
        ├── BUILD_INFO.json
        ├── SHA256SUMS
        ├── libgdble.windows.x86_64.dll
        ├── libgdble.linux.x86_64.so
        ├── libgdble.macos.x86_64.dylib
        ├── libgdble.macos.arm64.dylib
        └── android/
            ├── gdble-release.aar
            └── btleplug-release.aar
```

### Android 权限与导出

插件只声明权限，不自动请求：

- API 23–30：`BLUETOOTH`、`BLUETOOTH_ADMIN`、`ACCESS_FINE_LOCATION`
- API 31+：`BLUETOOTH_SCAN`、`BLUETOOTH_CONNECT`

应用必须先检查授权，并在权限回调后确认所需权限全部满足，才能调用 `BluetoothManager.initialize()`。两份 Demo 复用 `permission_startup.gd`：拒绝时保持未初始化并提示，返回前台只重新检查，不循环弹窗。Android 发行物是 `gdble-release.aar` 与独立的 `btleplug-release.aar`。

Android 导出需启用 Gradle Build，minSdk 23、compileSdk 34；配置步骤见 [Android 构建指南](docs/ANDROID_BUILD.md)。

### 从源码构建

`addons/gdble` 是唯一发行源。将整个目录复制到 Godot 项目的 `addons/gdble`。Demo 的 addon 副本由构建脚本生成，不在仓库中维护第二份。

桌面构建：

```powershell
.\build.ps1
```

```bash
./build.sh
```

Android 还需要 JDK 17、Android SDK 34、NDK 和 `cargo-ndk`，详见 [Android 构建](docs/ANDROID_BUILD.md)。

---

## 快速开始

### 基本使用流程

旧调用方式仍可使用；`connect_device()` 只创建/取得设备对象，不会隐式连接：

```gdscript
extends Node

var bluetooth: BluetoothManager
var device: BleDevice

func _ready() -> void:
    bluetooth = BluetoothManager.new()
    add_child(bluetooth)

    bluetooth.adapter_initialized.connect(_on_initialized)
    bluetooth.device_discovered.connect(_on_discovered)
    bluetooth.ble_event.connect(_on_ble_event)

    var startup = preload("res://addons/gdble/permission_startup.gd").new()
    add_child(startup)
    startup.permission_denied.connect(func(missing):
        print("蓝牙未初始化，请在系统设置中授权：", missing))
    startup.start(bluetooth)

func _on_initialized(success: bool, error: String) -> void:
    if success:
        bluetooth.start_scan(10.0)
    else:
        push_error(error)

func _on_discovered(info: Dictionary) -> void:
    if info.get("name") == "My Device":
        bluetooth.stop_scan()
        device = bluetooth.get_or_create_device(info.address)
        device.connected.connect(func(): device.discover_services())
        device.connect_async()

func _on_ble_event(event: Dictionary) -> void:
    if event.terminal:
        print(event.operation_id, " ", event.operation, " -> ", event.phase)
```

`start_scan(0.0)` 持续扫描，直到调用 `stop_scan()`；正数表示定时扫描；负数和非有限值返回 `INVALID_ARGUMENT`。重复开始扫描返回 `BUSY`，不会替换当前扫描。

---

## API 参考

### BluetoothManager

蓝牙管理器，负责适配器初始化、设备扫描和连接管理。

#### 方法

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `initialize()` | 无 | int | 初始化蓝牙适配器 |
| `is_initialized()` | 无 | bool | 检查适配器是否已初始化 |
| `get_adapter_info()` | 无 | Dictionary | 获取缓存的适配器信息 |
| `start_scan(timeout_seconds)` | float | int | 开始扫描设备 |
| `stop_scan()` | 无 | int | 停止扫描 |
| `get_discovered_devices()` | 无 | Array[Dictionary] | 获取已发现的设备列表 |
| `get_or_create_device(address)` | String | BleDevice | 创建或取得设备对象，不发起连接 |
| `connect_device(address)` | String | BleDevice | `get_or_create_device()` 的兼容别名 |
| `disconnect_device(address)` | String | int | 断开指定设备 |
| `get_device(address)` | String | BleDevice | 获取已有设备对象，可能尚未连接 |
| `get_connected_devices()` | 无 | Array[BleDevice] | 获取所有已连接设备 |
| `set_debug_mode(enabled)` | bool | void | 启用/禁用调试模式 |
| `is_debug_mode()` | 无 | bool | 检查调试模式状态 |

#### 信号

| 信号 | 参数 | 描述 |
|------|------|------|
| `ble_event` | event: Dictionary | 统一结构化事件 |
| `adapter_initialized` | success: bool, error: String | 适配器初始化完成 |
| `device_discovered` | device_info: Dictionary | 发现新设备 |
| `device_updated` | device_info: Dictionary | 设备信息更新 |
| `scan_started` | 无 | 扫描开始 |
| `scan_stopped` | 无 | 扫描停止 |
| `device_connecting` | address: String | 设备开始连接 |
| `device_connected` | address: String | 设备连接成功 |
| `device_disconnected` | address: String | 设备断开连接 |
| `error_occurred` | error_message: String | 发生错误 |


### BleDevice

表示单个 BLE 设备，提供连接、服务发现、数据读写等功能。

#### 方法

| 方法 | 参数 | 返回值 | 描述 |
|------|------|--------|------|
| `connect_async()` | 无 | int | 异步连接设备 |
| `disconnect()` | 无 | int | 断开设备连接 |
| `is_connected()` | 无 | bool | 检查是否已连接 |
| `get_address()` | 无 | String | 获取设备地址 |
| `get_name()` | 无 | String | 获取设备名称 |
| `discover_services()` | 无 | int | 发现设备服务 |
| `get_services()` | 无 | Array[Dictionary] | 获取已发现的服务列表 |
| `read_characteristic(service_uuid, char_uuid)` | String, String | int | 读取特征值 |
| `write_characteristic(service_uuid, char_uuid, data, with_response)` | String, String, PackedByteArray, bool | int | 写入特征值 |
| `subscribe_characteristic(service_uuid, char_uuid)` | String, String | int | 订阅特征通知 |
| `unsubscribe_characteristic(service_uuid, char_uuid)` | String, String | int | 取消订阅 |

#### 信号

| 信号 | 参数 | 描述 |
|------|------|------|
| `ble_event` | event: Dictionary | 统一结构化事件 |
| `connected` | 无 | 设备连接成功 |
| `disconnected` | 无 | 设备断开连接 |
| `connection_failed` | error: String | 连接失败 |
| `services_discovered` | services: Array | 服务发现完成 |
| `characteristic_read` | char_uuid: String, data: PackedByteArray | 特征读取完成 |
| `characteristic_written` | char_uuid: String | 特征写入完成 |
| `characteristic_notified` | char_uuid: String, data: PackedByteArray | 收到特征通知 |
| `operation_failed` | operation: String, error: String | 操作失败 |

同步 getter 只读取主线程缓存。`get_or_create_device()` 要求 Manager 已初始化；地址无效或未初始化时返回 `null`。

### 统一事件协议

`BluetoothManager` 和 `BleDevice` 都提供 `ble_event(event: Dictionary)`：

| 字段 | 类型 | 含义 |
| --- | --- | --- |
| `operation` | StringName | `initialize/scan/connect/disconnect/discover_services/read/write/subscribe/unsubscribe/notification` |
| `phase` | StringName | `started/progress/succeeded/failed/cancelled/received` |
| `operation_id` | int | 调用触发时为正数；远端断开等非请求事件为 `0` |
| `terminal` | bool | 该 operation ID 是否结束 |
| `device_address` | String | 规范化内部地址；无设备上下文时为空 |
| `service_uuid` | String | 小写标准 UUID；无 GATT 上下文时为空 |
| `characteristic_uuid` | String | 小写标准 UUID；无 GATT 上下文时为空 |
| `data` | Variant | Adapter、DeviceInfo、服务数组、字节或原因文本 |
| `error` | Dictionary | 固定包含 `code/message/retryable/details` |

原先返回 `void` 的异步方法现在返回可忽略的 operation ID。在 Manager 存活并持续处理事件期间，每个正 ID 恰好有一个终止事件；销毁 Manager 会取消后台工作，不再保证投递剩余事件。`stop_scan()` 返回当前扫描 ID；没有活动扫描时返回 `0`，命令队列满也不会丢失停止请求。

请先连接信号，再调用方法。参数错误、独立设备未初始化和命令队列拒绝会在方法返回前同步发出失败事件；不要仅依靠调用返回后才赋值的 ID 来接收这些事件。`BleDevice` 发起的本地拒绝由该设备发出，不会转发到 Manager。已接纳请求通过有界事件队列异步投递，设备事件由 Manager 转发至对应设备。缓存先更新，再发出结构化和旧版信号，回调中的同步 getter 可读取对应状态；请求被拒绝不会改变已有连接状态。

扫描 progress 事件的 DeviceInfo `data` 额外包含 `kind: StringName`，值为 `discovered` 或 `updated`。

常用新增错误码：`INVALID_ARGUMENT`、`NOT_INITIALIZED`、`BUSY`、`QUEUE_FULL`、`CANCELLED`、`ANDROID_NOT_INITIALIZED`、`AMBIGUOUS_CHARACTERISTIC`、`ADAPTER_UNAVAILABLE`、`EVENT_OVERFLOW`、`SERVICES_CHANGED`。旧错误信号继续输出兼容文本。

每个请求接纳前预留开始和终止事件容量，预留失败同步返回 `QUEUE_FULL`，不会执行后端操作。命令容量 64、外发事件容量 1024、每帧最多处理 256 项。控制事件与高频数据分开传递；扫描拥塞时按设备合并并淘汰最旧待发更新，报告 `EVENT_OVERFLOW`。非请求错误统一使用 `operation_id = 0`、`terminal = false`。反复的非请求状态变化在拥塞时可合并为最新状态，请求终止结果不会合并。

同一设备只使用一个 notification pump。订阅身份是完整 `(service_uuid, characteristic_uuid)`；btleplug 通知不携带 service UUID，因此跨服务订阅相同 characteristic UUID 会返回 `AMBIGUOUS_CHARACTERISTIC`。

通知接收器在后端订阅前准备就绪。订阅期间最多暂存 64 条、64 KiB，成功事件先发出，再按顺序投递数据。任一通知缓冲溢出会发出 `EVENT_OVERFLOW`（`error.details.source` 标明来源），停止该设备通知并清理未交付数据，保持连接；应用需要重新订阅。通知失败同时转发到旧 `operation_failed` 信号。

蓝牙关闭或状态未知时，`initialize` 非终止失败事件返回 `ADAPTER_UNAVAILABLE`，`error.details.state` 为 `powered_off` 或 `unknown`；初始化缓存立即失效，扫描和设备操作被取消。恢复供电时发送 `initialize` progress 提示，`data.reason = powered_on`，应用显式再次初始化。服务变化通过 `SERVICES_CHANGED` 使 GATT 缓存和订阅失效，需要重新发现服务后订阅。不会自动重连。

---

## 完整示例

可直接运行的 [demo](demo) 默认打开中文 BLE 调试工具：支持多设备连接、任意 GATT 服务浏览、HEX／UTF-8 收发、通知订阅和日志筛选，桌面分栏、手机分页。用 Godot 4.2+ 打开 `demo/project.godot` 后运行即可；权限和构建步骤见 [Demo 指南](demo/DEMO_GUIDE.md)。原有 [控制台示例](demo/examples/console.tscn) 和 [详细注释示例](demo/examples/commented.tscn) 仍可单独运行。以下代码示例假设平台蓝牙权限已授予；设备名和 UUID 请替换为目标设备的值。

### 示例 1: 扫描并连接设备

```gdscript
extends Node

var bluetooth_manager: BluetoothManager
var target_device_name = "MyDevice"

func _ready():
    bluetooth_manager = BluetoothManager.new()
    add_child(bluetooth_manager)

    bluetooth_manager.adapter_initialized.connect(_on_initialized)
    bluetooth_manager.device_discovered.connect(_on_device_found)
    bluetooth_manager.scan_stopped.connect(_on_scan_done)

    bluetooth_manager.initialize()

func _on_initialized(success: bool, error: String):
    if success:
        bluetooth_manager.start_scan(10.0)

func _on_device_found(info: Dictionary):
    var name: String = info.get("name") if info.get("name") != null else ""
    if name == target_device_name:
        print("找到目标设备!")
        bluetooth_manager.stop_scan()
        connect_to_target(info.get("address"))


func _on_scan_done():
    print("扫描完成")

func connect_to_target(address: String):
    var device = bluetooth_manager.connect_device(address)
    if device:
        device.connected.connect(_on_connected)
        device.connect_async()

func _on_connected():
    print("设备已连接!")
```

### 示例 2: 读取传感器数据

```gdscript
extends Node

var bluetooth_manager: BluetoothManager
var sensor_device: BleDevice

# 标准心率服务 UUID
const HEART_RATE_SERVICE = "0000180d-0000-1000-8000-00805f9b34fb"
const HEART_RATE_MEASUREMENT = "00002a37-0000-1000-8000-00805f9b34fb"

func _ready():
    bluetooth_manager = BluetoothManager.new()
    add_child(bluetooth_manager)

    bluetooth_manager.adapter_initialized.connect(_on_initialized)
    bluetooth_manager.device_discovered.connect(_on_device_found)

    bluetooth_manager.initialize()

func _on_initialized(success: bool, error: String):
    if success:
        bluetooth_manager.start_scan(10.0)

func _on_device_found(info: Dictionary):
    # 查找心率监测器
    var name: String = info.get("name") if info.get("name") != null else ""
    if "Heart" in name or "HR" in name:
        bluetooth_manager.stop_scan()
        connect_to_sensor(info.get("address"))

func connect_to_sensor(address: String):
    sensor_device = bluetooth_manager.connect_device(address)
    if sensor_device:
        sensor_device.connected.connect(_on_sensor_connected)
        sensor_device.services_discovered.connect(_on_services_found)
        sensor_device.characteristic_notified.connect(_on_heart_rate_update)
        sensor_device.connect_async()

func _on_sensor_connected():
    print("传感器已连接")
    sensor_device.discover_services()

func _on_services_found(services: Array):
    # 订阅心率通知
    sensor_device.subscribe_characteristic(HEART_RATE_SERVICE, HEART_RATE_MEASUREMENT)

func _on_heart_rate_update(char_uuid: String, data: PackedByteArray):
    if char_uuid.to_lower() == HEART_RATE_MEASUREMENT:
        # 解析心率数据（简化版）
        if data.size() > 1:
            var heart_rate = data[1]
            print("当前心率: ", heart_rate, " BPM")
```


### 示例 3: 控制智能灯泡

```gdscript
extends Node

var bluetooth_manager: BluetoothManager
var light_device: BleDevice

# 自定义服务 UUID（示例）
const LIGHT_SERVICE = "0000fff0-0000-1000-8000-00805f9b34fb"
const LIGHT_CONTROL = "0000fff2-0000-1000-8000-00805f9b34fb"

func _ready():
    bluetooth_manager = BluetoothManager.new()
    add_child(bluetooth_manager)

    bluetooth_manager.adapter_initialized.connect(_on_initialized)
    bluetooth_manager.device_discovered.connect(_on_light_found)
    bluetooth_manager.initialize()

func _on_initialized(success: bool, error: String):
    if success:
        bluetooth_manager.start_scan(10.0)

func _on_light_found(info: Dictionary):
    if info.get("name") == "MyLight":
        bluetooth_manager.stop_scan()
        connect_to_light(info.address)

func connect_to_light(address: String):
    light_device = bluetooth_manager.connect_device(address)
    if light_device:
        light_device.connected.connect(_on_light_connected)
        light_device.services_discovered.connect(_on_services_found)
        light_device.characteristic_written.connect(_on_command_sent)
        light_device.connect_async()

func _on_light_connected():
    print("灯泡已连接")
    light_device.discover_services()

func _on_services_found(services: Array):
    print("服务发现完成，可以控制灯泡了")

func set_light_color(red: int, green: int, blue: int):
    # 构造颜色命令（示例格式）
    var command = PackedByteArray([0x01, red, green, blue])
    light_device.write_characteristic(LIGHT_SERVICE, LIGHT_CONTROL, command, false)

func turn_on():
    var command = PackedByteArray([0x02, 0x01])
    light_device.write_characteristic(LIGHT_SERVICE, LIGHT_CONTROL, command, false)

func turn_off():
    var command = PackedByteArray([0x02, 0x00])
    light_device.write_characteristic(LIGHT_SERVICE, LIGHT_CONTROL, command, false)

func _on_command_sent(char_uuid: String):
    print("命令已发送")

# 在 UI 中调用
func _on_red_button_pressed():
    set_light_color(255, 0, 0)

func _on_green_button_pressed():
    set_light_color(0, 255, 0)

func _on_blue_button_pressed():
    set_light_color(0, 0, 255)
```

---

## 调试模式

保留 `set_debug_mode()` / `is_debug_mode()` 兼容接口。诊断操作结果时，建议同时记录 Manager 和设备的 `ble_event`：

```gdscript
bluetooth_manager.set_debug_mode(OS.is_debug_build())
bluetooth_manager.ble_event.connect(_on_ble_event)
# 取得设备后、发起操作前连接设备信号
device.ble_event.connect(_on_ble_event)

func _on_ble_event(event: Dictionary):
    print(event.operation_id, " ", event.operation, " -> ", event.phase)
    if event.phase == &"failed" or event.phase == &"cancelled":
        push_warning(str(event.error))
```

关闭兼容调试开关：`bluetooth_manager.set_debug_mode(false)`。上例中的自定义日志由应用控制。

---

## 常见问题

### Q: 为什么扫描不到设备？

**A:** 请检查：

1. 确保蓝牙适配器已启用
2. 确保目标设备处于广播状态
3. 检查设备是否在范围内（通常 10 米以内）
4. 启用调试模式查看详细日志

### Q: 连接设备失败怎么办？

**A:** 可能的原因：

1. 设备已被其他应用连接
2. 设备超出范围
3. 设备电量不足
4. 需要配对但未配对

### Q: 如何处理设备断开？

**A:** 监听 `disconnected` 信号：

```gdscript
device.disconnected.connect(_on_device_disconnected)

func _on_device_disconnected():
    print("设备断开，尝试重连...")
    # 实现重连逻辑
```

### Q: 写入数据没有响应？

**A:** 检查：

1. 特征是否支持写入（查看 `properties.write`）
2. 数据格式是否正确
3. 是否需要使用 `with_response` 参数
4. 启用调试模式查看错误信息

### Q: 如何找到正确的服务和特征 UUID？

**A:**

1. 查看设备文档或规格说明
2. 监听 `services_discovered` 查看所有服务
3. 使用标准 BLE 服务（如心率、电池等）
4. 使用第三方 BLE 扫描工具（如 nRF Connect）

### Q: 支持同时连接多个设备吗？

**A:** 是的，GDBLE 支持同时管理多个设备连接：

```gdscript
var device1 = bluetooth_manager.connect_device(address1)
var device2 = bluetooth_manager.connect_device(address2)
if device1:
    device1.connect_async()
if device2:
    device2.connect_async()
```


### Q: 性能如何？会影响游戏帧率吗？

**A:** GDBLE 使用异步架构，BLE 请求在后台执行，Manager 每帧最多消费 256 个排队事件。信号回调中的业务代码仍由应用负责控制耗时；销毁 Manager 时会等待有时限的清理。

### Q: 为什么只监听 Manager 会漏掉设备的参数错误？

设备发起的本地拒绝会在该设备上同步发出，调用前应连接设备的 `ble_event`；已接纳操作通过 Manager 异步投递。

---

## 数据格式说明

### DeviceInfo Dictionary

`name`、`rssi` 和 `tx_power_level` 可以为 `null`。以下为字段类型说明：

```gdscript
{
    "address": String,
    "name": String | null,
    "rssi": int | null,
    "services": Array[String],
    "manufacturer_data": Dictionary,
    "service_data": Dictionary,
    "tx_power_level": int | null,
}
```

### Service Dictionary

服务字典包含以下字段：

```gdscript
{
    "uuid": String,                    # 服务 UUID
    "characteristics": Array[Dictionary]  # 特征列表
}
```

### Characteristic Dictionary

特征字典包含以下字段：

```gdscript
{
    "uuid": String,        # 特征 UUID
    "properties": {        # 特征属性
        "read": bool,                    # 是否可读
        "write": bool,                   # 是否可写（需响应）
        "write_without_response": bool,  # 是否可写（无响应）
        "notify": bool,                  # 是否支持通知
        "indicate": bool                 # 是否支持指示
    }
}
```

---

## 最佳实践

### 1. 错误处理

始终处理错误情况：

```gdscript
bluetooth_manager.adapter_initialized.connect(func(success, error):
    if not success:
        push_error("蓝牙初始化失败: " + error)
        # 显示错误提示给用户
)

device.connection_failed.connect(func(error):
    push_error("连接失败: " + error)
    # 尝试重连或提示用户
)
```

### 2. 资源清理

在节点销毁时清理资源：

```gdscript
func _exit_tree():
    if device and device.is_connected():
        device.disconnect()
    if bluetooth_manager:
        bluetooth_manager.stop_scan()
```

### 3. 超时处理

后端初始化、扫描启停和设备操作各有 30 秒超时，超时返回 `TIMEOUT`；后端 Rust panic 转为 `INTERNAL_ERROR`。该时限从后端执行开始计算，不包含排队和事件投递时间，也不限制持续扫描或通知流的正常寿命。关闭时先取消任务，再在总计 2 秒的预算内尝试停止扫描、断开连接；运行时另有 100 毫秒退出宽限。底层系统调用若阻塞线程，或操作已在系统中生效，取消 future 并不保证撤销其物理效果。

适配器事件流异常结束会清空初始化状态、取消扫描与设备工作，并发出 `operation_id = 0`、`operation = initialize` 的非终止失败事件；完成有时限的清理后可再次调用 `initialize()`。通知流异常结束会发出同类 `notification` 失败事件，重新订阅可重建通知流。旧会话迟到的事件会被忽略，不会自动重连。

### 4. 状态管理

维护清晰的连接状态：

```gdscript
enum DeviceState {
    DISCONNECTED,
    CONNECTING,
    CONNECTED,
    DISCOVERING_SERVICES,
    READY
}

var device_state = DeviceState.DISCONNECTED

func connect_device(address: String):
    device_state = DeviceState.CONNECTING
    device = bluetooth_manager.connect_device(address)
    # ...

func _on_connected():
    device_state = DeviceState.CONNECTED
    device.discover_services()
    device_state = DeviceState.DISCOVERING_SERVICES

func _on_services_discovered(services: Array):
    device_state = DeviceState.READY
    # 现在可以进行读写操作
```

### 5. 数据验证

验证接收到的数据：

```gdscript
func _on_characteristic_read(char_uuid: String, data: PackedByteArray):
    if data.size() == 0:
        push_warning("收到空数据")
        return

    if data.size() < expected_size:
        push_warning("数据长度不足")
        return

    # 处理数据
    process_data(data)
```

---

## 性能优化建议

### 1. 扫描优化

```gdscript
# 使用合适的扫描时间
bluetooth_manager.start_scan(5.0)  # 5秒通常足够

# 找到目标设备后立即停止扫描
func _on_device_discovered(info: Dictionary):
    if info.get("name") == target_name:
        bluetooth_manager.stop_scan()
```

### 2. 写入优化

```gdscript
# 对于不需要确认的数据，使用无响应写入
device.write_characteristic(service, char, data, false)  # 更快

# 对于重要数据，使用有响应写入
device.write_characteristic(service, char, data, true)   # 更可靠
```

### 3. 通知优化

```gdscript
# 只订阅需要的特征
device.subscribe_characteristic(service, char)

# 不再需要时取消订阅
device.unsubscribe_characteristic(service, char)
```

---

## 故障排除

### 启用详细日志

```gdscript
bluetooth_manager.set_debug_mode(true)
```

### 检查适配器状态

```gdscript
if not bluetooth_manager.is_initialized():
    print("适配器未初始化")
```

### 验证设备连接

```gdscript
if device and device.is_connected():
    print("设备已连接")
else:
    print("设备未连接")
```

### 查看发现的服务

```gdscript
func _on_services_discovered(services: Array):
    print("发现的服务:")
    for service in services:
        print("  服务: ", service.get("uuid"))
        for char in service.get("characteristics", []):
            print("    特征: ", char.get("uuid"))
            print("    属性: ", char.get("properties"))
```

---

## 贡献

欢迎贡献代码、报告问题或提出建议！

- 报告问题: [GitHub Issues](https://github.com/Fantety/GDBLE/issues)
- 提交代码: [Pull Requests](https://github.com/Fantety/GDBLE/pulls)

### 开发验证与发布

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
```

架构词汇见 [CONTEXT.md](CONTEXT.md)，关键决策见 [docs/adr](docs/adr)。问题与贡献请使用 [Issues](https://github.com/Fantety/GDBLE/issues) 和 [Pull Requests](https://github.com/Fantety/GDBLE/pulls)。

发布维护者请遵循 [两阶段发布流程](docs/RELEASING.md)。每个版本的发布说明保存在 [release-notes](release-notes)，GitHub Actions 根据 tag 读取对应 Markdown 文件，无需逐次修改 workflow。

---

## 许可证

本项目采用 MIT 许可证。详见 [LICENSE](LICENSE) 文件。

---

## 致谢

- [btleplug](https://github.com/deviceplug/btleplug) - 优秀的 Rust BLE 库
- [godot-rust](https://github.com/godot-rust/gdext) - Godot Rust 绑定
- Godot 社区的支持和反馈

---

## 联系方式

- GitHub: [@Fantety](https://github.com/Fantety)
- 项目主页: [GDBLE](https://github.com/Fantety/GDBLE)

---

<div align="center">
  <p>如果这个项目对你有帮助，请给个 ⭐️ Star！</p>
  <p>Made with ❤️ for Godot Community</p>
</div>
