# 需求文档

## 简介

本项目旨在为 Godot 4 游戏引擎开发一个蓝牙低功耗（BLE）插件，通过封装 Rust 的 btleplug 库，使 GDScript 开发者能够在 Godot 项目中轻松使用蓝牙功能。该插件将提供设备扫描、连接管理、特征读写等核心 BLE 功能。

## 术语表

- **GDBLE系统**: 本项目开发的 Godot 蓝牙插件系统
- **BLE设备**: 蓝牙低功耗（Bluetooth Low Energy）外围设备
- **中心设备**: 发起 BLE 连接的设备（本插件运行的设备）
- **外围设备**: 被连接的 BLE 设备
- **适配器**: 系统的蓝牙硬件适配器
- **特征值**: BLE GATT 协议中的数据单元，具有唯一 UUID
- **服务**: BLE GATT 协议中的服务，包含多个特征值
- **GDScript**: Godot 引擎的脚本语言
- **GDExtension**: Godot 4 的原生扩展系统

## 需求

### 需求 1：蓝牙适配器管理

**用户故事：** 作为 Godot 开发者，我希望能够初始化和管理蓝牙适配器，以便开始使用蓝牙功能

#### 验收标准

1. THE GDBLE系统 SHALL 提供一个 BluetoothManager 节点类用于管理蓝牙适配器
2. WHEN GDScript 调用初始化方法时，THE GDBLE系统 SHALL 获取系统默认蓝牙适配器
3. IF 系统没有可用的蓝牙适配器，THEN THE GDBLE系统 SHALL 通过信号通知初始化失败
4. THE GDBLE系统 SHALL 提供方法检查适配器是否已初始化
5. THE GDBLE系统 SHALL 提供方法获取适配器信息（名称、地址等）

### 需求 2：BLE 设备扫描

**用户故事：** 作为 Godot 开发者，我希望能够扫描附近的 BLE 设备，以便发现可连接的外围设备

#### 验收标准

1. THE GDBLE系统 SHALL 提供方法启动 BLE 设备扫描
2. THE GDBLE系统 SHALL 提供方法停止 BLE 设备扫描
3. WHEN 发现新的 BLE设备 时，THE GDBLE系统 SHALL 通过信号发送设备信息（名称、地址、信号强度）
4. THE GDBLE系统 SHALL 支持设置扫描超时时间
5. WHILE 扫描进行中，THE GDBLE系统 SHALL 持续更新已发现设备的信号强度
6. THE GDBLE系统 SHALL 提供方法获取所有已发现设备的列表

### 需求 3：设备连接管理

**用户故事：** 作为 Godot 开发者，我希望能够连接和断开 BLE 设备，以便与外围设备进行通信

#### 验收标准

1. THE GDBLE系统 SHALL 提供方法通过设备地址连接到 BLE设备
2. WHEN 连接成功时，THE GDBLE系统 SHALL 通过信号通知连接状态
3. WHEN 连接失败时，THE GDBLE系统 SHALL 通过信号通知失败原因
4. THE GDBLE系统 SHALL 提供方法断开已连接的 BLE设备
5. WHEN 设备意外断开时，THE GDBLE系统 SHALL 通过信号通知断开事件
6. THE GDBLE系统 SHALL 提供方法检查设备连接状态
7. THE GDBLE系统 SHALL 支持同时管理多个设备连接

### 需求 4：服务和特征值发现

**用户故事：** 作为 Godot 开发者，我希望能够发现已连接设备的服务和特征值，以便了解设备支持的功能

#### 验收标准

1. WHEN 设备连接成功后，THE GDBLE系统 SHALL 自动发现设备的所有服务
2. THE GDBLE系统 SHALL 提供方法获取设备的所有服务列表
3. THE GDBLE系统 SHALL 提供方法获取指定服务的所有特征值列表
4. THE GDBLE系统 SHALL 提供特征值的属性信息（可读、可写、可通知等）
5. WHEN 服务发现完成时，THE GDBLE系统 SHALL 通过信号通知发现结果

### 需求 5：特征值读写操作

**用户故事：** 作为 Godot 开发者，我希望能够读写 BLE 特征值，以便与设备交换数据

#### 验收标准

1. THE GDBLE系统 SHALL 提供方法通过 UUID 读取特征值数据
2. THE GDBLE系统 SHALL 提供方法通过 UUID 写入特征值数据
3. WHEN 读取操作完成时，THE GDBLE系统 SHALL 通过信号返回读取的字节数据
4. WHEN 写入操作完成时，THE GDBLE系统 SHALL 通过信号通知写入结果
5. IF 读写操作失败，THEN THE GDBLE系统 SHALL 通过信号返回错误信息
6. THE GDBLE系统 SHALL 支持写入操作的响应和无响应模式

### 需求 6：特征值通知订阅

**用户故事：** 作为 Godot 开发者，我希望能够订阅特征值的通知，以便实时接收设备推送的数据

#### 验收标准

1. THE GDBLE系统 SHALL 提供方法订阅特征值通知
2. THE GDBLE系统 SHALL 提供方法取消订阅特征值通知
3. WHEN 收到特征值通知时，THE GDBLE系统 SHALL 通过信号发送通知数据
4. THE GDBLE系统 SHALL 支持同时订阅多个特征值
5. WHEN 取消订阅成功时，THE GDBLE系统 SHALL 通过信号确认取消操作

### 需求 7：错误处理和日志

**用户故事：** 作为 Godot 开发者，我希望插件能够提供清晰的错误信息和日志，以便调试和排查问题

#### 验收标准

1. THE GDBLE系统 SHALL 为所有异步操作提供错误回调信号
2. THE GDBLE系统 SHALL 将错误信息输出到 Godot 控制台
3. THE GDBLE系统 SHALL 提供错误代码和描述性错误消息
4. WHERE 调试模式启用，THE GDBLE系统 SHALL 输出详细的操作日志
5. THE GDBLE系统 SHALL 处理所有 btleplug 库的异常并转换为 Godot 友好的错误信息

### 需求 8：跨平台支持

**用户故事：** 作为 Godot 开发者，我希望插件能够在多个平台上工作，以便开发跨平台应用

#### 验收标准

1. THE GDBLE系统 SHALL 支持 Windows 平台的蓝牙功能
2. THE GDBLE系统 SHALL 支持 Linux 平台的蓝牙功能
3. THE GDBLE系统 SHALL 支持 macOS 平台的蓝牙功能
4. WHERE 平台不支持蓝牙功能，THE GDBLE系统 SHALL 在初始化时返回明确的错误信息
5. THE GDBLE系统 SHALL 提供统一的 API 接口，隐藏平台差异

### 需求 9：资源管理和生命周期

**用户故事：** 作为 Godot 开发者，我希望插件能够正确管理资源，以便避免内存泄漏和资源冲突

#### 验收标准

1. WHEN BluetoothManager 节点被释放时，THE GDBLE系统 SHALL 自动断开所有连接
2. WHEN BluetoothManager 节点被释放时，THE GDBLE系统 SHALL 停止所有正在进行的扫描
3. THE GDBLE系统 SHALL 正确释放所有 Rust 端的资源
4. THE GDBLE系统 SHALL 使用 Godot 的引用计数系统管理设备对象
5. THE GDBLE系统 SHALL 在后台线程中执行阻塞操作，避免阻塞主线程
