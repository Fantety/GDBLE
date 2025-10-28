# 实现计划

- [x] 1. 设置项目基础结构和共享类型




  - 更新 Cargo.toml 添加 tokio 依赖
  - 创建 types.rs 定义共享数据结构（DeviceInfo, BleError 等）
  - 实现 Rust 类型到 Godot Dictionary 的转换方法
  - _需求: 1.1, 1.2, 7.3_

- [x] 2. 实现 Tokio 运行时管理器





  - 创建 runtime.rs 文件
  - 实现 RuntimeManager struct 用于管理 Tokio 运行时
  - 提供 spawn 和 block_on 方法用于执行异步任务
  - _需求: 9.5_

- [x] 3. 实现 BluetoothManager 核心节点





  - 创建 bluetooth_manager.rs 文件
  - 定义 BluetoothManager GodotClass 继承 Node
  - 实现 initialize 方法获取系统蓝牙适配器
  - 实现 is_initialized 和 get_adapter_info 方法
  - 定义所有必需的 Godot 信号（adapter_initialized, error_occurred 等）
  - 实现 ready 和 on_notification 生命周期方法
  - _需求: 1.1, 1.2, 1.3, 1.4, 1.5_

- [x] 4. 实现设备扫描功能





  - 创建 bluetooth_scanner.rs 文件
  - 实现 BluetoothScanner struct（非 GodotClass）
  - 实现 start_scan 异步方法，使用 btleplug 的 scan API
  - 实现 stop_scan 方法
  - 实现设备发现回调，收集 DeviceInfo
  - 在 BluetoothManager 中集成扫描器
  - 实现 start_scan, stop_scan, get_discovered_devices 方法
  - 通过信号发送设备发现事件到 GDScript
  - _需求: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6_

- [x] 5. 实现 BLE 服务和特征值信息结构





  - 创建 ble_service.rs 文件
  - 定义 BleServiceInfo struct 存储服务 UUID 和特征值列表
  - 创建 ble_characteristic.rs 文件
  - 定义 BleCharacteristicInfo 和 CharacteristicProperties struct
  - 实现到 Dictionary 的转换方法
  - _需求: 4.2, 4.3, 4.4_

- [x] 6. 实现 BleDevice 设备封装





  - 创建 ble_device.rs 文件
  - 定义 BleDevice GodotClass 继承 RefCounted
  - 实现构造函数，接收 Peripheral 和 Runtime
  - 实现 connect_async 方法连接设备
  - 实现 disconnect 方法断开连接
  - 实现 is_connected, get_address, get_name 方法
  - 定义所有设备相关信号（connected, disconnected, connection_failed 等）
  - 处理连接状态变化并发射相应信号
  - _需求: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6_

- [x] 7. 实现服务发现功能





  - 在 BleDevice 中实现 discover_services 方法
  - 使用 btleplug 的 discover_services API
  - 将发现的服务转换为 BleServiceInfo
  - 实现 get_services 方法返回服务列表
  - 发射 services_discovered 信号
  - _需求: 4.1, 4.2, 4.3, 4.4, 4.5_

- [x] 8. 实现特征值读写操作





  - 在 BleDevice 中实现 read_characteristic 方法
  - 通过 service_uuid 和 char_uuid 查找特征值
  - 使用 btleplug 的 read API 读取数据
  - 实现 write_characteristic 方法
  - 支持 with_response 和 without_response 写入模式
  - 通过信号返回读取的数据和写入结果
  - 实现错误处理，通过 operation_failed 信号返回错误
  - _需求: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_

- [x] 9. 实现特征值通知订阅





  - 在 BleDevice 中实现 subscribe_characteristic 方法
  - 使用 btleplug 的 subscribe API
  - 设置通知回调处理器
  - 实现 unsubscribe_characteristic 方法
  - 在收到通知时通过 characteristic_notified 信号发送数据
  - 支持同时订阅多个特征值
  - _需求: 6.1, 6.2, 6.3, 6.4, 6.5_

- [x] 10. 实现多设备连接管理




  - 在 BluetoothManager 中维护设备映射表（HashMap）
  - 实现 connect_device 方法创建 BleDevice 实例
  - 实现 disconnect_device 方法
  - 处理设备断开事件，更新设备映射表
  - 确保设备对象的正确引用计数
  - _需求: 3.7_

- [x] 11. 完善错误处理和日志系统





  - 在 types.rs 中完善 BleError 枚举
  - 实现 BleError 到字符串的转换
  - 在所有异步操作中添加错误处理
  - 使用 godot_error! 和 godot_warn! 宏输出日志
  - 实现调试模式的详细日志输出
  - 确保所有错误都通过信号传递到 GDScript
  - _需求: 7.1, 7.2, 7.3, 7.4, 7.5_

- [x] 12. 实现资源清理和生命周期管理





  - 在 BluetoothManager 中实现 on_notification(NOTIFICATION_PREDELETE)
  - 在节点销毁时停止所有扫描
  - 断开所有设备连接
  - 释放 Tokio 运行时资源
  - 在 BleDevice 中实现 Drop trait 或 on_notification
  - 确保设备断开时清理所有订阅
  - _需求: 9.1, 9.2, 9.3, 9.4_

- [x] 13. 更新 lib.rs 注册所有类





  - 在 lib.rs 中导入所有模块
  - 确保 BluetoothManager 和 BleDevice 正确注册到 GDExtension
  - 修复 crate 命名警告（使用 snake_case）
  - _需求: 所有_

- [ ] 14. 创建 GDExtension 配置文件





  - 创建 gdble.gdextension 配置文件
  - 配置各平台的库路径
  - 设置 entry_symbol 和兼容性版本
  - _需求: 8.1, 8.2, 8.3, 8.5_

- [ ]* 15. 创建 GDScript 测试场景
  - 创建测试场景验证适配器初始化
  - 测试设备扫描功能
  - 测试设备连接和断开
  - 测试特征值读写
  - 测试通知订阅
  - _需求: 所有_

- [ ]* 16. 编写使用文档和示例
  - 创建 README.md 说明插件安装和使用
  - 编写 GDScript 使用示例
  - 说明各平台的权限要求
  - 提供常见问题解答
  - _需求: 8.4_
