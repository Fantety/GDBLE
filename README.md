# GDBLE 0.6.0

面向 Godot 4.2+ 的 Rust BLE GDExtension，基于 btleplug 0.12.0。支持扫描、连接、GATT 服务发现、读写和通知，并保留 0.5.x 的类名、方法参数与旧信号。

- 项目：[github.com/Fantety/GDBLE](https://github.com/Fantety/GDBLE)
- Asset Library：[GDBLE](https://godotengine.org/asset-library/asset/3439)
- 许可证：[MIT](LICENSE)

## 平台

| 平台 | 架构 | 最低要求 |
| --- | --- | --- |
| Windows | x86_64 | Windows 10 |
| Linux | x86_64 | 发行版需提供 BlueZ/DBus |
| macOS | x86_64、ARM64 | macOS 10.15 / 11 |
| Android | ARM64、x86_64 | API 23，compileSdk 34 |

不支持 Android ARMv7、iOS 和自动重连。Android ARM64 用于真机，x86_64 主要用于模拟器加载测试。

## 安装

`addons/gdble` 是唯一发行源。将整个目录复制到 Godot 项目的 `addons/gdble`。Demo 的 addon 副本由构建脚本生成，不在仓库中维护第二份。

桌面构建：

```powershell
.\build.ps1
```

```bash
./build.sh
```

Android 还需要 JDK 17、Android SDK 34、NDK 和 `cargo-ndk`，详见 [Android 构建](docs/ANDROID_BUILD.md)。

## 快速开始

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

    if OS.get_name() == "Android":
        OS.request_permissions()

    var initialize_id := bluetooth.initialize()
    print("initialize operation: ", initialize_id)

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

## 统一事件协议

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

常用新增错误码：`INVALID_ARGUMENT`、`NOT_INITIALIZED`、`BUSY`、`QUEUE_FULL`、`CANCELLED`、`ANDROID_NOT_INITIALIZED`、`AMBIGUOUS_CHARACTERISTIC`。旧错误信号继续输出兼容文本。

后端初始化、扫描启停和设备操作各有 30 秒超时，超时返回 `TIMEOUT`；后端 Rust panic 转为 `INTERNAL_ERROR`。该时限从后端执行开始计算，不包含排队和事件投递时间，也不限制持续扫描或通知流的正常寿命。关闭时先取消任务，再在总计 2 秒的预算内尝试停止扫描、断开连接；运行时另有 100 毫秒退出宽限。底层系统调用若阻塞线程，或操作已在系统中生效，取消 future 并不保证撤销其物理效果。

适配器事件流异常结束会清空初始化状态、取消扫描与设备工作，并发出 `operation_id = 0`、`operation = initialize` 的非终止失败事件；完成有时限的清理后可再次调用 `initialize()`。通知流异常结束会发出同类 `notification` 失败事件，重新订阅可重建通知流。旧会话迟到的事件会被忽略，不会自动重连。

## API 兼容层

### BluetoothManager

- `initialize() -> int`
- `start_scan(timeout_seconds: float) -> int`
- `stop_scan() -> int`
- `get_discovered_devices() -> Array[Dictionary]`
- `get_or_create_device(address: String) -> BleDevice`
- `connect_device(address: String) -> BleDevice`：兼容别名，不连接
- `disconnect_device(address: String) -> int`
- `get_device(address: String) -> BleDevice`
- `get_connected_devices() -> Array[BleDevice]`

### BleDevice

- `connect_async()/disconnect()/discover_services() -> int`
- `read_characteristic(service_uuid, characteristic_uuid) -> int`
- `write_characteristic(service_uuid, characteristic_uuid, data, with_response) -> int`
- `subscribe_characteristic(service_uuid, characteristic_uuid) -> int`
- `unsubscribe_characteristic(service_uuid, characteristic_uuid) -> int`
- `is_connected()`、`get_services()`、`get_address()`、`get_name()` 只读主线程缓存。

同一设备只使用一个 notification pump。订阅身份是完整 `(service_uuid, characteristic_uuid)`；btleplug 通知不携带 service UUID，因此跨服务订阅相同 characteristic UUID 会返回 `AMBIGUOUS_CHARACTERISTIC`。

## DeviceInfo

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

## Android 权限

插件只声明权限，不自动请求：

- API 23–30：`BLUETOOTH`、`BLUETOOTH_ADMIN`、`ACCESS_FINE_LOCATION`
- API 31+：`BLUETOOTH_SCAN`、`BLUETOOTH_CONNECT`

应用应在 `BluetoothManager.initialize()` 前调用 `OS.request_permissions()`。Android 发行物是 `gdble-release.aar` 与独立的 `btleplug-release.aar`，不是 raw `.so` 或 fat AAR。

## 开发验证

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo test --all-targets --all-features --locked
```

架构词汇见 [CONTEXT.md](CONTEXT.md)，关键决策见 [docs/adr](docs/adr)。问题与贡献请使用 [Issues](https://github.com/Fantety/GDBLE/issues) 和 [Pull Requests](https://github.com/Fantety/GDBLE/pulls)。

发布维护者请遵循 [两阶段发布流程](docs/RELEASING.md)。每个版本的发布说明保存在 [release-notes](release-notes)，GitHub Actions 根据 tag 读取对应 Markdown 文件，无需逐次修改 workflow。
