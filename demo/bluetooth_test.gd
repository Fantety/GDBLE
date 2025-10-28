extends Node

# Bluetooth 插件测试脚本
# 演示如何使用 GDBle 插件进行蓝牙设备扫描和连接

var bluetooth_manager: BluetoothManager
var connected_device: BleDevice = null

func _ready():
	print("=== Bluetooth Plugin Test ===")
	
	# 创建 BluetoothManager 实例
	bluetooth_manager = BluetoothManager.new()
	add_child(bluetooth_manager)
	
	# 连接信号 (只在初始化时连接一次)
	bluetooth_manager.adapter_initialized.connect(_on_adapter_initialized)
	bluetooth_manager.device_discovered.connect(_on_device_discovered)
	bluetooth_manager.device_connected.connect(_on_device_connected)
	bluetooth_manager.device_disconnected.connect(_on_device_disconnected)
	bluetooth_manager.scan_started.connect(_on_scan_started)
	bluetooth_manager.scan_stopped.connect(_on_scan_stopped)
	bluetooth_manager.error_occurred.connect(_on_error_occurred)
	
	# 初始化蓝牙适配器
	print("Initializing Bluetooth adapter...")
	# 关闭调试模式以减少输出
	bluetooth_manager.set_debug_mode(false)
	bluetooth_manager.initialize()
	#start_scanning()

func start_scanning():
	print("\n=== Starting BLE Scan ===")
	
	# 开始扫描（扫描 10 秒）
	bluetooth_manager.start_scan(10.0)

func connect_to_device(address: String):
	print("\n=== Connecting to device: ", address, " ===")
	
	# 通过 BluetoothManager 连接设备
	var device = bluetooth_manager.connect_device(address)
	if device:
		print("Device instance created successfully")
		connected_device = device
		
		# 连接设备信号
		print("Connecting device signals...")
		device.connected.connect(_on_device_connected_signal)
		device.disconnected.connect(_on_device_disconnected_signal)
		device.connection_failed.connect(_on_connection_failed)
		device.services_discovered.connect(_on_services_discovered)
		device.characteristic_read.connect(_on_characteristic_read)
		device.characteristic_written.connect(_on_characteristic_written)
		device.characteristic_notified.connect(_on_characteristic_notified)
		device.operation_failed.connect(_on_operation_failed)
		
		print("Signals connected, starting async connection...")
		# 开始连接
		device.connect_async()
		print("Connection process initiated")
	else:
		print("Failed to create device instance")

func discover_services():
	if connected_device:
		print("\n=== Discovering services ===")
		print("Device address: ", connected_device.get_address())
		print("Device name: ", connected_device.get_name())
		print("Is connected: ", connected_device.is_connected())
		print("Calling discover_services()...")
		connected_device.discover_services()
	else:
		print("No connected device available for service discovery")

func read_characteristic_example(service_uuid: String, char_uuid: String):
	if connected_device:
		print("\n=== Reading characteristic ===")
		print("  Service: ", service_uuid)
		print("  Characteristic: ", char_uuid)
		connected_device.read_characteristic(service_uuid, char_uuid)

func write_characteristic_example(service_uuid: String, char_uuid: String, data: PackedByteArray, with_response: bool = true):
	if connected_device:
		print("\n=== Writing characteristic ===")
		print("  Service: ", service_uuid)
		print("  Characteristic: ", char_uuid)
		print("  Data length: ", data.size())
		connected_device.write_characteristic(service_uuid, char_uuid, data, with_response)

func subscribe_characteristic_example(service_uuid: String, char_uuid: String):
	if connected_device:
		print("\n=== Subscribing to characteristic ===")
		print("  Service: ", service_uuid)
		print("  Characteristic: ", char_uuid)
		connected_device.subscribe_characteristic(service_uuid, char_uuid)

func unsubscribe_characteristic_example(service_uuid: String, char_uuid: String):
	if connected_device:
		print("\n=== Unsubscribing from characteristic ===")
		print("  Service: ", service_uuid)
		print("  Characteristic: ", char_uuid)
		connected_device.unsubscribe_characteristic(service_uuid, char_uuid)

# ===== 信号回调函数 =====

func _on_adapter_initialized(success: bool, error: String):
	if success:
		print("Bluetooth adapter initialized successfully")
		# 初始化成功后开始扫描
		start_scanning()
	else:
		print("Failed to initialize Bluetooth adapter: ", error)

func _on_scan_started():
	print("Scan started")

func _on_scan_stopped():
	print("Scan stopped")
	
	# 获取所有发现的设备
	var devices = bluetooth_manager.get_discovered_devices()
	print("\nTotal devices discovered: ", devices.size())
	
	# 搜索并连接到"Fantety11"设备
	var target_address = ""
	for device in devices:
		var name = device.get("name", "")
		if name == "Fantety11":
			target_address = device.get("address", "")
			break
	
	if target_address != "":
		print("Found target device: Fantety11 at ", target_address)
		connect_to_device(target_address)
	elif devices.size() > 0 and connected_device == null:
		# 如果没找到目标设备，连接到第一个设备
		var first_device = devices[0]
		var address = first_device.get("address", "")
		var name = first_device.get("name", "")
		if address != "":
			print("Connecting to first available device: ", name, " at ", address)
			connect_to_device(address)
	else:
		print("No devices found to connect to")

func _on_device_discovered(device_info: Dictionary):
	print("\nDevice discovered:")
	print("  Name: ", device_info.get("name", "Unknown"))
	print("  Address: ", device_info.get("address", ""))
	print("  RSSI: ", device_info.get("rssi", 0), " dBm")

func _on_device_connected(address: String):
	print("Device connected (manager signal): ", address)

func _on_device_disconnected(address: String):
	print("Device disconnected (manager signal): ", address)
	connected_device = null

func _on_device_connected_signal():
	print("Device connected successfully")
	# 连接成功后发现服务
	print("Calling discover_services()...")
	discover_services()

func _on_device_disconnected_signal():
	print("Device disconnected")
	connected_device = null

func _on_connection_failed(error: String):
	print("Connection failed: ", error)
	connected_device = null

func _on_services_discovered(services: Array):
	print("\n=== Services discovered callback ===")
	print("Received services array, size: ", services.size())
	print("Services data: ", services)
	
	if services.size() == 0:
		print("No services discovered")
		return
		
	print("\nServices discovered:")
	for service in services:
		var service_uuid = service.get("uuid", "")
		print("  Service UUID: ", service_uuid)
		
		var characteristics = service.get("characteristics", [])
		print("    Characteristics count: ", characteristics.size())
		for characteristic in characteristics:
			var char_uuid = characteristic.get("uuid", "")
			var properties = characteristic.get("properties", {})
			print("    Characteristic UUID: ", char_uuid)
			print("      Can Read: ", properties.get("read", false))
			print("      Can Write: ", properties.get("write", false))
			print("      Can Write Without Response: ", properties.get("write_without_response", false))
			print("      Can Notify: ", properties.get("notify", false))
			print("      Can Indicate: ", properties.get("indicate", false))
	
	# 查找fff0服务并处理其特征
	var fff0_service_found = false
	var fff1_subscribed = false
	var fff2_written = false
	
	for service in services:
		var service_uuid = service.get("uuid", "")
		var characteristics = service.get("characteristics", [])
		
		print("Checking service: ", service_uuid)
		
		# 检查是否是fff0服务（UUID格式：0000fff0-0000-1000-8000-00805f9b34fb）
		if service_uuid == "0000fff0-0000-1000-8000-00805f9b34fb":
			print("\n=== Found fff0 service: ", service_uuid, " ===")
			fff0_service_found = true
			
			for characteristic in characteristics:
				var char_uuid = characteristic.get("uuid", "")
				var properties = characteristic.get("properties", {})
				print("  Checking characteristic: ", char_uuid)
				
				# 订阅 fff1 特征的通知
				if char_uuid == "0000fff1-0000-1000-8000-00805f9b34fb":
					print("  Found fff1 characteristic: ", char_uuid)
					if properties.get("notify", false):
						print("  Subscribing to fff1 notifications...")
						subscribe_characteristic_example(service_uuid, char_uuid)
						fff1_subscribed = true
					else:
						print("  Warning: fff1 does not support notifications")
				
				# 写入数据到 fff2 特征
				elif char_uuid == "0000fff2-0000-1000-8000-00805f9b34fb":
					print("  Found fff2 characteristic: ", char_uuid)
					if properties.get("write", false) or properties.get("write_without_response", false):
						# 写入字符串 "hello gdble"
						var test_string = "hello gdble"
						var test_data = test_string.to_utf8_buffer()
						print("  Writing string '", test_string, "' to fff2 characteristic")
						write_characteristic_example(service_uuid, char_uuid, test_data, false)
						fff2_written = true
					else:
						print("  Warning: fff2 does not support write operations")
			
			# 找到 fff0 服务后就退出循环
			break
	
	# 输出操作结果摘要
	if fff0_service_found:
		print("\n=== fff0 Service Operations Summary ===")
		print("  fff1 notification subscribed: ", fff1_subscribed)
		print("  fff2 data written: ", fff2_written)
	else:
		print("\nWarning: fff0 service not found")


func _on_characteristic_read(char_uuid: String, data: PackedByteArray):
	print("\nCharacteristic read:")
	print("  UUID: ", char_uuid)
	print("  Data length: ", data.size())
	print("  Data (hex): ", data.hex_encode())

func _on_characteristic_written(char_uuid: String):
	print("\nCharacteristic written successfully:")
	print("  UUID: ", char_uuid)

func _on_characteristic_notified(char_uuid: String, data: PackedByteArray):
	print("\n=== Characteristic Notification Received ===")
	print("  UUID: ", char_uuid)
	print("  Data length: ", data.size())
	print("  Data (hex): ", data.hex_encode())
	
	# 尝试将数据解析为字符串
	var data_string = data.get_string_from_utf8()
	if data_string != "":
		print("  Data (string): ", data_string)
	
	# 特别标记来自 fff1 的通知
	if char_uuid.to_lower() == "0000fff1-0000-1000-8000-00805f9b34fb":
		print("  >>> This is from fff1 characteristic! <<<")

func _on_operation_failed(operation: String, error: String):
	print("\nOperation failed:")
	print("  Operation: ", operation)
	print("  Error: ", error)

func _on_error_occurred(error_message: String):
	print("\nError occurred: ", error_message)

func _exit_tree():
	# 清理资源
	if connected_device:
		connected_device.disconnect()
	if bluetooth_manager:
		bluetooth_manager.stop_scan()
