# 异步执行修复说明

## 问题描述

之前的实现使用 `block_on` 同步执行所有蓝牙操作，导致：
- 所有输出被缓冲，直到操作完成才一次性显示
- Godot 主线程被阻塞，影响用户体验
- 无法实时看到扫描进度和连接状态

## 解决方案

将所有蓝牙操作从 `block_on` 改为 `spawn` 异步执行：

### 修改的方法

1. **BluetoothManager::start_scan** - 扫描操作
2. **BleDevice::connect_async** - 设备连接
3. **BleDevice::discover_services** - 服务发现
4. **BleDevice::read_characteristic** - 读取特征值
5. **BleDevice::write_characteristic** - 写入特征值
6. **BleDevice::subscribe_characteristic** - 订阅通知
7. **BleDevice::unsubscribe_characteristic** - 取消订阅

### 关键改动

**之前 (使用 block_on):**
```rust
let result = runtime.block_on(async {
    peripheral.connect().await
});
// 处理结果...
```

**现在 (使用 spawn):**
```rust
runtime.spawn(async move {
    let result = peripheral.connect().await;
    // 在异步任务中处理结果
    // 使用 call_deferred 在主线程发射信号
    if let Ok(mut obj) = Gd::<BleDevice>::try_from_instance_id(instance_id) {
        obj.call_deferred("_on_connect_success", &[]);
    }
});
```

### 核心原理

1. **异步执行**: 使用 `runtime.spawn()` 在后台线程池执行蓝牙操作
2. **非阻塞**: 主线程立即返回，不等待操作完成
3. **信号回调**: 通过 `call_deferred` 在 Godot 主线程安全地发射信号
4. **实时输出**: 日志输出不再被缓冲，可以实时看到进度

## 测试方法

1. 编译项目:
   ```bash
   cargo build --release
   ```

2. 复制 DLL 到 demo 目录:
   ```bash
   copy target\release\gdble.dll demo\addons\gdble\gdble.dll
   ```

3. 在 Godot 中运行 `demo/bluetooth_test.gd`

4. 观察输出 - 现在应该能实时看到:
   - 扫描开始提示
   - 每个发现的设备信息
   - 连接进度
   - 服务发现过程
   - 数据读写操作

## 预期效果

- ✅ 输出实时显示，不再等到最后一次性输出
- ✅ Godot 界面保持响应
- ✅ 可以看到扫描和连接的实时进度
- ✅ 所有功能正常工作（扫描、连接、读写、通知）

## 注意事项

- 所有信号发射都通过 `call_deferred` 确保在主线程执行
- 使用 `instance_id` 在异步任务中安全地访问 Godot 对象
- 保持了原有的错误处理和日志记录机制
