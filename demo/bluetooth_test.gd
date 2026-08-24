extends Node

# Bluetooth 插件测试脚本
# 演示如何使用 GDBle 插件进行蓝牙设备扫描和连接
# 测试线程安全的事件传递机制

var bluetooth_manager: BluetoothManager
var connected_device: BleDevice = null

# 测试统计
var test_stats = {
	"connect_count": 0,
	"disconnect_count": 0,
	"service_discovery_count": 0,
	"read_count": 0,
	"write_count": 0,
	"notification_count": 0,
	"error_count": 0,
	"subscribe_count": 0,
	"unsubscribe_count": 0
}

# 测试配置
var test_config = {
	"enable_stress_test": false,
	"stress_test_iterations": 3,
	"auto_disconnect_after_test": false,
	"disconnect_delay": 5.0
}

var stress_test_iteration = 0
var stress_test_timer: float = 0.0
var is_stress_testing: bool = false

func _ready():
	print("=== Bluetooth Plugin Test (Thread-Safe Event System) ===")
	print("Testing channel-based event communication")
	
	bluetooth_manager = BluetoothManager.new()
	add_child(bluetooth_manager)
	
	bluetooth_manager.adapter_initialized.connect(_on_adapter_initialized)
	bluetooth_manager.device_discovered.connect(_on_device_discovered)
	bluetooth_manager.device_connected.connect(_on_device_connected)
	bluetooth_manager.device_disconnected.connect(_on_device_disconnected)
	bluetooth_manager.scan_started.connect(_on_scan_started)
	bluetooth_manager.scan_stopped.connect(_on_scan_stopped)
	bluetooth_manager.error_occurred.connect(_on_error_occurred)
	bluetooth_manager.ble_event.connect(_on_ble_event)
	
	print("Initializing Bluetooth adapter...")
	bluetooth_manager.set_debug_mode(true)
	if OS.get_name() == "Android":
		OS.request_permissions()
	bluetooth_manager.initialize()

func _on_ble_event(event: Dictionary):
	if event.terminal or event.phase == &"received":
		print("BLE event: ", event)

func _process(delta: float):
	if is_stress_testing:
		stress_test_timer += delta

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
		test_stats.subscribe_count += 1
		print("\n[TEST] Subscribing to characteristic (count: ", test_stats.subscribe_count, ")")
		print("  Service: ", service_uuid)
		print("  Characteristic: ", char_uuid)
		connected_device.subscribe_characteristic(service_uuid, char_uuid)

func unsubscribe_characteristic_example(service_uuid: String, char_uuid: String):
	if connected_device:
		test_stats.unsubscribe_count += 1
		print("\n[TEST] Unsubscribing from characteristic (count: ", test_stats.unsubscribe_count, ")")
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
		if name == "Fantety的Mate 80":
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
	test_stats.connect_count += 1
	print("\n[TEST] Device connected successfully (count: ", test_stats.connect_count, ")")
	print("[TEST] This signal was delivered via thread-safe channel")
	discover_services()

func _on_device_disconnected_signal():
	test_stats.disconnect_count += 1
	print("\n[TEST] Device disconnected (count: ", test_stats.disconnect_count, ")")
	print("[TEST] Disconnection event processed via channel")
	connected_device = null
	
	if is_stress_testing:
		_handle_stress_test_disconnect()

func _on_connection_failed(error: String):
	test_stats.error_count += 1
	print("\n[TEST] Connection failed: ", error, " (error count: ", test_stats.error_count, ")")
	connected_device = null

func _on_services_discovered(services: Array):
	test_stats.service_discovery_count += 1
	print("\n[TEST] Services discovered (count: ", test_stats.service_discovery_count, ")")
	print("[TEST] Services array size: ", services.size())
	
	if services.size() == 0:
		print("[TEST] No services discovered")
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
	
	var fff0_service_found = false
	var fff1_subscribed = false
	var fff2_written = false
	
	for service in services:
		var service_uuid = service.get("uuid", "")
		var characteristics = service.get("characteristics", [])
		
		if service_uuid == "0000fff0-0000-1000-8000-00805f9b34fb":
			print("\n[TEST] Found fff0 service: ", service_uuid)
			fff0_service_found = true
			
			for characteristic in characteristics:
				var char_uuid = characteristic.get("uuid", "")
				var properties = characteristic.get("properties", {})
				
				if char_uuid == "0000fff1-0000-1000-8000-00805f9b34fb":
					if properties.get("notify", false):
						print("[TEST] Subscribing to fff1 notifications...")
						subscribe_characteristic_example(service_uuid, char_uuid)
						fff1_subscribed = true
				
				elif char_uuid == "0000fff2-0000-1000-8000-00805f9b34fb":
					if properties.get("write", false) or properties.get("write_without_response", false):
						var test_string = "hello gdble"
						var test_data = test_string.to_utf8_buffer()
						print("[TEST] Writing '", test_string, "' to fff2 characteristic")
						write_characteristic_example(service_uuid, char_uuid, test_data, false)
						fff2_written = true
			
			break
	
	if fff0_service_found:
		print("\n[TEST] fff0 Service Operations Summary:")
		print("  fff1 notification subscribed: ", fff1_subscribed)
		print("  fff2 data written: ", fff2_written)
	else:
		print("\n[TEST] fff0 service not found, testing with first available characteristic")
		_test_first_available_characteristic(services)


func _test_first_available_characteristic(services: Array):
	for service in services:
		var service_uuid = service.get("uuid", "")
		var characteristics = service.get("characteristics", [])
		
		for characteristic in characteristics:
			var char_uuid = characteristic.get("uuid", "")
			var properties = characteristic.get("properties", {})
			
			if properties.get("read", false):
				print("[TEST] Testing read on: ", char_uuid)
				read_characteristic_example(service_uuid, char_uuid)
				return
			
			if properties.get("notify", false):
				print("[TEST] Testing subscribe on: ", char_uuid)
				subscribe_characteristic_example(service_uuid, char_uuid)
				return


func _on_characteristic_read(char_uuid: String, data: PackedByteArray):
	test_stats.read_count += 1
	print("\n[TEST] Characteristic read (count: ", test_stats.read_count, ")")
	print("  UUID: ", char_uuid)
	print("  Data length: ", data.size())
	print("  Data (hex): ", data.hex_encode())

func _on_characteristic_written(char_uuid: String):
	test_stats.write_count += 1
	print("\n[TEST] Characteristic written (count: ", test_stats.write_count, ")")
	print("  UUID: ", char_uuid)

func _on_characteristic_notified(char_uuid: String, data: PackedByteArray):
	test_stats.notification_count += 1
	print("\n[TEST] Characteristic notification (count: ", test_stats.notification_count, ")")
	print("  UUID: ", char_uuid)
	print("  Data length: ", data.size())
	print("  Data (hex): ", data.hex_encode())
	
	var data_string = data.get_string_from_utf8()
	if data_string != "":
		print("  Data (string): ", data_string)
	
	if char_uuid.to_lower() == "0000fff1-0000-1000-8000-00805f9b34fb":
		print("  >>> This is from fff1 characteristic! <<<")

func _on_operation_failed(operation: String, error: String):
	test_stats.error_count += 1
	print("\n[TEST] Operation failed (error count: ", test_stats.error_count, ")")
	print("  Operation: ", operation)
	print("  Error: ", error)

func _on_error_occurred(error_message: String):
	test_stats.error_count += 1
	print("\n[TEST] Error occurred (error count: ", test_stats.error_count, "): ", error_message)

func _exit_tree():
	_print_test_summary()
	if connected_device:
		connected_device.disconnect()
	if bluetooth_manager:
		bluetooth_manager.stop_scan()


func _print_test_summary():
	print("\n" + "=".repeat(50))
	print("=== TEST SUMMARY (Thread-Safe Event System) ===")
	print("=".repeat(50))
	print("Connect operations: ", test_stats.connect_count)
	print("Disconnect operations: ", test_stats.disconnect_count)
	print("Service discoveries: ", test_stats.service_discovery_count)
	print("Characteristic reads: ", test_stats.read_count)
	print("Characteristic writes: ", test_stats.write_count)
	print("Characteristic notifications: ", test_stats.notification_count)
	print("Subscribe operations: ", test_stats.subscribe_count)
	print("Unsubscribe operations: ", test_stats.unsubscribe_count)
	print("Errors encountered: ", test_stats.error_count)
	print("=".repeat(50))
	
	if test_stats.error_count == 0:
		print("[TEST] All operations completed successfully!")
		print("[TEST] Thread-safe channel communication working correctly!")
	else:
		print("[TEST] Some errors occurred during testing")
	print("=".repeat(50))


func _handle_stress_test_disconnect():
	stress_test_iteration += 1
	print("\n[STRESS TEST] Iteration ", stress_test_iteration, "/", test_config.stress_test_iterations)
	
	if stress_test_iteration >= test_config.stress_test_iterations:
		print("[STRESS TEST] Completed all iterations!")
		is_stress_testing = false
		_print_test_summary()
		return
	
	await get_tree().create_timer(1.0).timeout
	print("[STRESS TEST] Starting next iteration...")
	start_scanning()


func start_stress_test():
	print("\n" + "=".repeat(50))
	print("=== STARTING STRESS TEST ===")
	print("Iterations: ", test_config.stress_test_iterations)
	print("=".repeat(50))
	
	is_stress_testing = true
	stress_test_iteration = 0
	test_stats = {
		"connect_count": 0,
		"disconnect_count": 0,
		"service_discovery_count": 0,
		"read_count": 0,
		"write_count": 0,
		"notification_count": 0,
		"error_count": 0,
		"subscribe_count": 0,
		"unsubscribe_count": 0
	}
	
	start_scanning()


func _input(event: InputEvent):
	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_F1:
				print("\n[INPUT] Starting stress test...")
				test_config.enable_stress_test = true
				start_stress_test()
			KEY_F2:
				print("\n[INPUT] Printing current test stats...")
				_print_test_summary()
			KEY_F3:
				if connected_device:
					print("\n[INPUT] Disconnecting device...")
					connected_device.disconnect()
			KEY_F4:
				if connected_device:
					print("\n[INPUT] Reconnecting...")
					connected_device.connect_async()
