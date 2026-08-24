# GDBLE Demo 使用指南

Demo 要求 Godot 4.2+。`demo/addons/gdble` 由根目录构建脚本从 canonical `addons/gdble` 生成；首次运行前请执行 `build.ps1` 或 `build.sh`。

Android 导出使用 Gradle、minSdk 23，并同时打包 GDBLE 与 btleplug 两个 AAR。Demo 会在 `BluetoothManager.initialize()` 前调用 `OS.request_permissions()`。插件不会自行弹出权限框。

两个示例同时监听旧信号和新的 `ble_event(Dictionary)`；异步方法的返回值是可用于关联终止事件的 operation ID，旧代码可以忽略该返回值。

## 文件说明

### bluetooth_test.gd
基本测试脚本，演示完整的蓝牙操作流程：
- 扫描设备
- 连接到 "Fantety11" 设备
- 订阅 fff1 特征的通知
- 写入数据到 fff2 特征

### bluetooth_test_with_comments.gd
详细注释版本，包含：
- 完整的代码注释
- 清晰的操作流程说明
- 更友好的输出格式
- 推荐用于学习和参考

## 快速开始

### 1. 准备工作

确保你的设备：
- 已开启蓝牙
- 设备名称为 "Fantety11"（或修改脚本中的目标设备名）
- 包含 fff0 服务及其特征：
  - fff1: 用于发送通知（设备 → Godot）
  - fff2: 用于接收数据（Godot → 设备）

### 2. 运行测试

1. 在 Godot 中打开 demo 场景
2. 将测试脚本附加到节点
3. 运行场景
4. 观察控制台输出

### 3. 预期输出

```
=== GDBLE Bluetooth Plugin Test ===
Initializing Bluetooth adapter...
✓ Bluetooth adapter initialized

=== Starting BLE Device Scan ===
✓ Scan started
  Device: Fantety11 (XX:XX:XX:XX:XX:XX) RSSI: -45 dBm
✓ Scan stopped

=== Scan Results ===
  Total devices found: 1
  ✓ Found target device: Fantety11

=== Connecting to Device ===
  Address: XX:XX:XX:XX:XX:XX
  Device instance created
  Setting up device signals...
  Initiating connection...
✓ Device connected successfully

=== Discovering Services ===
  Device: Fantety11
  Address: XX:XX:XX:XX:XX:XX
  Connected: true

=== Services Discovered ===
  Total services: 8

=== Processing fff0 Service ===
  Service UUID: 0000fff0-0000-1000-8000-00805f9b34fb
  Characteristics: 3

  [fff1] Notification Characteristic
    UUID: 0000fff1-0000-1000-8000-00805f9b34fb
    Can Notify: true
    >>> Subscribing to notifications...

  [fff2] Write Characteristic
    UUID: 0000fff2-0000-1000-8000-00805f9b34fb
    Can Write: true
    >>> Writing test data...

=== Operation Summary ===
  ✓ fff0 service found
  fff1 subscribed: ✓
  fff2 written: ✓

>>> Waiting for notifications from fff1...
>>> Any data received will be displayed below

=== Characteristic Written ===
  UUID: 0000fff2-0000-1000-8000-00805f9b34fb
  ✓ Data successfully written to fff2

============================================================
=== NOTIFICATION RECEIVED ===
============================================================
  UUID: 0000fff1-0000-1000-8000-00805f9b34fb
  Length: 11 bytes
  Hex: 68656c6c6f206764626c65
  Text: 'hello gdble'

  >>> This notification is from fff1 characteristic! <<<
  >>> Your device sent this data <<<
============================================================
```

## 自定义修改

### 修改目标设备

在脚本中找到并修改：

```gdscript
# 原代码
if name == "Fantety11":

# 改为你的设备名
if name == "YourDeviceName":
```

### 修改服务和特征 UUID

```gdscript
# 在脚本顶部定义
const YOUR_SERVICE = "your-service-uuid"
const YOUR_CHAR_NOTIFY = "your-notify-char-uuid"
const YOUR_CHAR_WRITE = "your-write-char-uuid"
```

### 修改写入的数据

```gdscript
# 在 _on_services_discovered 函数中
write_to_fff2("your custom message")
```

### 启用调试模式

查看详细的内部日志：

```gdscript
# 在 _ready() 函数中
bluetooth_manager.set_debug_mode(true)  # 改为 true
```

## 常见问题

### Q: 找不到设备？
**A:** 检查：
- 设备是否开启并处于广播状态
- 设备名称是否正确
- 蓝牙适配器是否启用
- 设备是否在范围内（10米以内）

### Q: 连接失败？
**A:** 可能原因：
- 设备已被其他应用连接
- 设备需要配对
- 信号太弱
- 设备电量不足

### Q: 收不到通知？
**A:** 检查：
- 是否成功订阅了特征
- 特征是否支持通知（Can Notify: true）
- 设备是否真的发送了数据
- 启用调试模式查看详细日志

### Q: 写入失败？
**A:** 检查：
- 特征是否支持写入
- 数据格式是否正确
- 是否已连接到设备
- 查看 operation_failed 信号的错误信息

## 测试流程图

```
开始
  ↓
初始化蓝牙适配器
  ↓
扫描设备 (10秒)
  ↓
找到目标设备？
  ├─ 是 → 连接设备
  └─ 否 → 结束
       ↓
    连接成功？
       ├─ 是 → 发现服务
       └─ 否 → 结束
            ↓
         找到 fff0 服务？
            ├─ 是 → 处理特征
            └─ 否 → 结束
                 ↓
              订阅 fff1 通知
                 ↓
              写入数据到 fff2
                 ↓
              等待通知
                 ↓
              收到数据！
```

## 进阶使用

### 1. 读取特征值

```gdscript
func read_characteristic_example():
    if connected_device:
        connected_device.read_characteristic(
            FFF0_SERVICE,
            FFF3_CHARACTERISTIC  # 假设 fff3 支持读取
        )
```

### 2. 取消订阅

```gdscript
func unsubscribe_from_fff1():
    if connected_device:
        connected_device.unsubscribe_characteristic(
            FFF0_SERVICE,
            FFF1_CHARACTERISTIC
        )
```

### 3. 发送二进制数据

```gdscript
func send_binary_data():
    var data = PackedByteArray([0x01, 0x02, 0x03, 0x04])
    connected_device.write_characteristic(
        FFF0_SERVICE,
        FFF2_CHARACTERISTIC,
        data,
        false  # 无响应写入
    )
```

### 4. 处理多种数据格式

```gdscript
func _on_characteristic_notified(char_uuid: String, data: PackedByteArray):
    # 尝试解析为文本
    var text = data.get_string_from_utf8()
    if text != "":
        print("Text: ", text)
        return
    
    # 解析为整数
    if data.size() == 4:
        var value = data.decode_u32(0)
        print("Integer: ", value)
        return
    
    # 解析为浮点数
    if data.size() == 4:
        var value = data.decode_float(0)
        print("Float: ", value)
        return
    
    # 其他格式，显示十六进制
    print("Hex: ", data.hex_encode())
```

## 性能提示

1. **扫描时间**: 根据需要调整扫描时间，通常 5-10 秒足够
2. **写入模式**: 对于不需要确认的数据，使用无响应写入（更快）
3. **通知频率**: 避免设备发送过于频繁的通知（建议 < 100Hz）
4. **连接管理**: 不使用时及时断开连接，节省电量

## 调试技巧

1. **启用调试模式**:
   ```gdscript
   bluetooth_manager.set_debug_mode(true)
   ```

2. **打印所有服务**:
   ```gdscript
   for service in services:
       print("Service: ", service.get("uuid"))
       for char in service.get("characteristics", []):
           print("  Char: ", char.get("uuid"))
           print("  Props: ", char.get("properties"))
   ```

3. **使用第三方工具**:
   - nRF Connect (移动端)
   - Bluetooth LE Explorer (Windows)
   - LightBlue (iOS/macOS)

## 支持

如有问题，请：
1. 查看 README.md 中的完整文档
2. 启用调试模式查看详细日志
3. 在 GitHub 提交 Issue

---

祝你使用愉快！🎮📱
