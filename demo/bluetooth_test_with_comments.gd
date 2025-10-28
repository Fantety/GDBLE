extends Node

# ============================================================================
# Bluetooth 插件测试脚本 - 详细注释版
# ============================================================================
# 
# 本脚本演示如何使用 GDBLE 插件进行蓝牙设备操作：
# 1. 初始化蓝牙适配器
# 2. 扫描附近的 BLE 设备
# 3. 连接到目标设备（Fantety11）
# 4. 发现设备的服务和特征
# 5. 订阅 fff1 特征的通知（接收数据）
# 6. 写入数据到 fff2 特征（发送数据）
#
# ============================================================================

var bluetooth_manager: BluetoothManager
var connected_device: BleDevice = null

# fff0 服务的 UUID
const FFF0_SERVICE = "0000fff0-0000-1000-8000-00805f9b34fb"
# fff1 特征的 UUID（用于接收通知）
const FFF1_CHARACTERISTIC = "0000fff1-0000-1000-8000-00805f9b34fb"
# fff2 特征的 UUID（用于写入数据）
const FFF2_CHARACTERISTIC = "0000fff2-0000-1000-8000-00805f9b34fb"

func _ready():
	print("=== GDBLE Bluetooth Plugin Test ===")
	print("This script will:")
	print("  1. Initialize Bluetooth adapter")
	print("  2. Scan for devices")
	print("  3. Connect to 'Fantety11' device")
	print("  4. Subscribe to fff1 notifications")
	print("  5. Write data to fff2 characteristic")
	print()
	
	# 步骤 1: 创建 BluetoothManager 实例
	bluetooth_manager = BluetoothManager.new()
	add_child(bluetooth_manager)
	
	# 步骤 2: 连接所有必要的信号
	setup_signals()
	
	# 步骤 3: 初始化蓝牙适配器
	print("Initializing Bluetooth adapter...")
	# 关闭调试模式以保持输出清洁（开发时可设为 true）
	bluetooth_manager.set_debug_mode(false)
	bluetooth_manager.initialize()

func setup_signals():
	"""连接所有蓝牙管理器和设备的信号"""
	# BluetoothManager 信号
	bluetooth_manager.adapter_initialized.connect(_on_adapter_initialized)
	bluetooth_manager.device_discovered.connect(_on_device_discovered)
	bluetooth_manager.device_connected.connect(_on_device_connected)
	bluetooth_manager.device_disconnected.connect(_on_device_disconnected)
	bluetooth_manager.scan_started.connect(_on_scan_started)
	bluetooth_manager.scan_stopped.connect(_on_scan_stopped)
	bluetooth_manager.error_occurred.connect(_on_error_occurred)

func start_scanning():
	"""开始扫描附近的 BLE 设备"""
	print("\n=== Starting BLE Device Scan ===")
	# 扫描 10 秒
	bluetooth_manager.start_scan(10.0)

func connect_to_device(address: String):
	"""连接到指定地址的 BLE 设备"""
	print("\n=== Connecting to Device ===")
	print("  Address: ", address)
	
	# 通过 BluetoothManager 创建设备实例
	var device = bluetooth_manager.connect_device(address)
	if device:
		print("  Device instance created")
		connected_device = device
		
		# 连接设备的所有信号
		print("  Setting up device signals...")
		device.connected.connect(_on_device_connected_signal)
		device.disconnected.connect(_on_device_disconnected_signal)
		device.connection_failed.connect(_on_connection_failed)
		device.services_discovered.connect(_on_services_discovered)
		device.characteristic_read.connect(_on_characteristic_read)
		device.characteristic_written.connect(_on_characteristic_written)
		device.characteristic_notified.connect(_on_characteristic_notified)
		device.operation_failed.connect(_on_operation_failed)
		
		# 开始异步连接
		print("  Initiating connection...")
		device.connect_async()
	else:
		print("  ERROR: Failed to create device instance")

func discover_services():
	"""发现已连接设备的所有服务和特征"""
	if connected_device:
		print("\n=== Discovering Services ===")
		print("  Device: ", connected_device.get_name())
		print("  Address: ", connected_device.get_address())
		print("  Connected: ", connected_device.is_connected())
		connected_device.discover_services()
	else:
		print("ERROR: No connected device")

# ============================================================================
# 信号回调函数 - BluetoothManager
# ============================================================================

func _on_adapter_initialized(success: bool, error: String):
	"""蓝牙适配器初始化完成"""
	if success:
		print("✓ Bluetooth adapter initialized")
		# 初始化成功后自动开始扫描
		start_scanning()
	else:
		print("✗ Bluetooth initialization failed: ", error)

func _on_scan_started():
	"""扫描开始"""
	print("✓ Scan started")

func _on_scan_stopped():
	"""扫描停止，处理发现的设备"""
	print("✓ Scan stopped")
	
	# 获取所有发现的设备
	var devices = bluetooth_manager.get_discovered_devices()
	print("\n=== Scan Results ===")
	print("  Total devices found: ", devices.size())
	
	# 查找目标设备 "Fantety11"
	var target_address = ""
	for device in devices:
		var name = device.get("name", "")
		if name == "Fantety11":
			target_address = device.get("address", "")
			print("  ✓ Found target device: Fantety11")
			break
	
	# 连接到目标设备
	if target_address != "":
		connect_to_device(target_address)
	elif devices.size() > 0 and connected_device == null:
		# 如果没找到目标设备，连接到第一个设备（用于测试）
		var first_device = devices[0]
		var address = first_device.get("address", "")
		var name = first_device.get("name", "Unknown")
		print("  Target device not found, connecting to: ", name)
		connect_to_device(address)
	else:
		print("  ✗ No devices available to connect")

func _on_device_discovered(device_info: Dictionary):
	"""发现新设备时的回调"""
	var name = device_info.get("name", "Unknown")
	var address = device_info.get("address", "")
	var rssi = device_info.get("rssi", 0)
	print("  Device: ", name, " (", address, ") RSSI: ", rssi, " dBm")

func _on_device_connected(address: String):
	"""设备连接成功（管理器信号）"""
	print("✓ Device connected (manager): ", address)

func _on_device_disconnected(address: String):
	"""设备断开连接（管理器信号）"""
	print("✗ Device disconnected (manager): ", address)
	connected_device = null

func _on_error_occurred(error_message: String):
	"""发生错误"""
	print("✗ Error: ", error_message)

# ============================================================================
# 信号回调函数 - BleDevice
# ============================================================================

func _on_device_connected_signal():
	"""设备连接成功（设备信号）"""
	print("✓ Device connected successfully")
	# 连接成功后立即发现服务
	discover_services()

func _on_device_disconnected_signal():
	"""设备断开连接（设备信号）"""
	print("✗ Device disconnected")
	connected_device = null

func _on_connection_failed(error: String):
	"""连接失败"""
	print("✗ Connection failed: ", error)
	connected_device = null


func _on_services_discovered(services: Array):
	"""服务发现完成，处理 fff0 服务"""
	print("\n=== Services Discovered ===")
	print("  Total services: ", services.size())
	
	if services.size() == 0:
		print("  ✗ No services found")
		return
	
	# 查找并处理 fff0 服务
	var fff0_found = false
	var fff1_subscribed = false
	var fff2_written = false
	
	for service in services:
		var service_uuid = service.get("uuid", "")
		
		# 检查是否是 fff0 服务
		if service_uuid == FFF0_SERVICE:
			print("\n=== Processing fff0 Service ===")
			print("  Service UUID: ", service_uuid)
			fff0_found = true
			
			var characteristics = service.get("characteristics", [])
			print("  Characteristics: ", characteristics.size())
			
			# 遍历特征
			for characteristic in characteristics:
				var char_uuid = characteristic.get("uuid", "")
				var properties = characteristic.get("properties", {})
				
				# 处理 fff1 特征（订阅通知）
				if char_uuid == FFF1_CHARACTERISTIC:
					print("\n  [fff1] Notification Characteristic")
					print("    UUID: ", char_uuid)
					print("    Can Notify: ", properties.get("notify", false))
					
					if properties.get("notify", false):
						print("    >>> Subscribing to notifications...")
						subscribe_to_fff1()
						fff1_subscribed = true
					else:
						print("    ✗ Warning: Notifications not supported")
				
				# 处理 fff2 特征（写入数据）
				elif char_uuid == FFF2_CHARACTERISTIC:
					print("\n  [fff2] Write Characteristic")
					print("    UUID: ", char_uuid)
					print("    Can Write: ", properties.get("write", false))
					print("    Can Write Without Response: ", properties.get("write_without_response", false))
					
					if properties.get("write", false) or properties.get("write_without_response", false):
						print("    >>> Writing test data...")
						write_to_fff2("hello gdble")
						fff2_written = true
					else:
						print("    ✗ Warning: Write not supported")
			
			# 找到 fff0 服务后退出循环
			break
	
	# 输出操作摘要
	print("\n=== Operation Summary ===")
	if fff0_found:
		print("  ✓ fff0 service found")
		print("  fff1 subscribed: ", "✓" if fff1_subscribed else "✗")
		print("  fff2 written: ", "✓" if fff2_written else "✗")
		
		if fff1_subscribed:
			print("\n>>> Waiting for notifications from fff1...")
			print(">>> Any data received will be displayed below")
	else:
		print("  ✗ fff0 service not found")

func subscribe_to_fff1():
	"""订阅 fff1 特征的通知"""
	if connected_device:
		connected_device.subscribe_characteristic(FFF0_SERVICE, FFF1_CHARACTERISTIC)

func write_to_fff2(text: String):
	"""写入文本数据到 fff2 特征"""
	if connected_device:
		var data = text.to_utf8_buffer()
		# 使用无响应写入（更快）
		connected_device.write_characteristic(FFF0_SERVICE, FFF2_CHARACTERISTIC, data, false)

func _on_characteristic_read(char_uuid: String, data: PackedByteArray):
	"""特征读取完成"""
	print("\n=== Characteristic Read ===")
	print("  UUID: ", char_uuid)
	print("  Length: ", data.size(), " bytes")
	print("  Hex: ", data.hex_encode())
	
	var text = data.get_string_from_utf8()
	if text != "":
		print("  Text: ", text)

func _on_characteristic_written(char_uuid: String):
	"""特征写入完成"""
	print("\n=== Characteristic Written ===")
	print("  UUID: ", char_uuid)
	
	# 特别标记 fff2 的写入
	if char_uuid.to_lower() == FFF2_CHARACTERISTIC:
		print("  ✓ Data successfully written to fff2")

func _on_characteristic_notified(char_uuid: String, data: PackedByteArray):
	"""收到特征通知（这是接收数据的主要方式）"""
	print("\n" + "="*60)
	print("=== NOTIFICATION RECEIVED ===")
	print("="*60)
	print("  UUID: ", char_uuid)
	print("  Length: ", data.size(), " bytes")
	print("  Hex: ", data.hex_encode())
	
	# 尝试解析为文本
	var text = data.get_string_from_utf8()
	if text != "":
		print("  Text: '", text, "'")
	
	# 特别标记来自 fff1 的通知
	if char_uuid.to_lower() == FFF1_CHARACTERISTIC:
		print("\n  >>> This notification is from fff1 characteristic! <<<")
		print("  >>> Your device sent this data <<<")
	
	print("="*60)

func _on_operation_failed(operation: String, error: String):
	"""操作失败"""
	print("\n✗ Operation Failed")
	print("  Operation: ", operation)
	print("  Error: ", error)

# ============================================================================
# 清理函数
# ============================================================================

func _exit_tree():
	"""节点销毁时清理资源"""
	print("\n=== Cleaning Up ===")
	
	if connected_device:
		print("  Disconnecting device...")
		connected_device.disconnect()
	
	if bluetooth_manager:
		print("  Stopping scan...")
		bluetooth_manager.stop_scan()
	
	print("  Cleanup complete")
