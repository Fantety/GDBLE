extends SceneTree

const Controller = preload("res://debugger/controller.gd")
const DeviceAccess = preload("res://debugger/device_access.gd")
const Codec = preload("res://debugger/codec.gd")
const Fakes = preload("res://debugger/tests/fakes.gd")
const A := "AA:BB:CC:DD:EE:01"
const B := "AA:BB:CC:DD:EE:02"

var controller
var manager
var view

func _initialize() -> void:
	_run.call_deferred()

func tick() -> void:
	await process_frame
	await process_frame
	if view:
		view.refresh()
	await process_frame

func _run() -> void:
	create_timer(20.0).timeout.connect(func(): push_error("Debugger test timeout"); quit(1))
	await _native_and_permission_checks()
	manager = Fakes.Manager.new()
	controller = Controller.new()
	root.add_child(controller)
	controller.configure(manager, false, func(device): return device)
	view = load("res://node_2d.tscn").instantiate()
	view.controller = controller
	root.add_child(view)
	view.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	await tick()
	assert(controller.ready_for_ble and not controller.initializing)
	controller.start_scan(10)
	await tick()
	assert(controller.scanning and not controller.scan_pending)
	manager.advertise("  aa:bb:cc:dd:ee:01 ", "温度传感器", -65)
	manager.advertise(A, "温度传感器", -42)
	manager.advertise(B, "串口设备", -58)
	await tick()
	assert(controller.sessions.size() == 2 and controller.sessions[A].rssi == -42)
	assert(view.devices.devices.get_root().get_child_count() == 2)
	view.devices.filter.text = "串口"
	view.devices.refresh()
	assert(view.devices.count.text.begins_with("1 台设备"))
	view.devices.filter.text = "ee:01"
	view.devices.refresh()
	assert(view.devices.count.text.begins_with("1 台设备"))
	view.devices.filter.text = ""
	controller.stop_scan()
	await tick()
	assert(not controller.scanning and not controller.scan_pending)
	manager.reject_scan = true
	controller.start_scan(3)
	await tick()
	assert(not controller.scan_pending and not view.devices.scan.disabled)
	manager.reject_scan = false
	# Independent in-flight connections; switching never disconnects another device.
	for address in [A, B]:
		manager.get_or_create_device(address).delayed = true
		controller.connect_device(address)
	assert(controller.sessions[A].pending.has("connect") and controller.sessions[B].pending.has("connect"))
	for address in [A, B]:
		var device = manager.devices[address]
		device.delayed = false
		device.complete(device.waiting.pop_front())
	await tick()
	assert(controller.sessions[A].connected and controller.sessions[B].connected)
	assert(controller.sessions[A].services.size() == 2)
	assert(manager.devices[A].calls.size() == 2) # Only connect and service discovery.
	controller.select_device(A)
	controller.select_characteristic(Fakes.SERVICE_A, Fakes.CHARACTERISTIC)
	controller.current().draft = "01 AF\n 20"
	await tick()
	assert(controller.current().with_response)
	assert(not view.gatt.read_button.disabled and not view.gatt.send.disabled)
	var before: int = manager.devices[A].calls.size()
	for invalid in ["A", "01 G2", "0x12", "12-34"]:
		view.gatt.input.text = invalid
		view.gatt._input_changed()
		assert(view.gatt.send.disabled and not view.gatt.validation.text.is_empty())
		assert(not controller.write().ok)
	assert(manager.devices[A].calls.size() == before)
	view.gatt.input.text = "01 af\t20\n"
	view.gatt._input_changed()
	assert(controller.write().ok)
	await tick()
	assert(manager.devices[A].calls.back().data == PackedByteArray([1, 175, 32]))
	assert(manager.devices[A].calls.back().response)
	controller.current().format = "UTF-8"
	controller.current().draft = "你好 BLE"
	controller.write()
	await tick()
	assert(manager.devices[A].calls.back().data == "你好 BLE".to_utf8_buffer())
	controller.select_device(B)
	controller.select_characteristic(Fakes.SERVICE_B, Fakes.CHARACTERISTIC)
	controller.current().draft = "02"
	assert(not controller.current().with_response)
	controller.write()
	await tick()
	assert(not manager.devices[B].calls.back().response)
	controller.select_device(A)
	await tick()
	assert(controller.current().draft == "你好 BLE" and view.gatt.input.text == "你好 BLE")
	# The manager forwards device events, but exactly one log is added per read.
	before = controller.logs.entries.size()
	controller.read()
	await tick()
	assert(controller.logs.entries.size() == before + 1)
	assert("A中" in view.gatt.latest.text and "41 E4 B8 AD" in view.gatt.latest.text)
	var key_a: String = controller.current().key(Fakes.SERVICE_A, Fakes.CHARACTERISTIC)
	manager.devices[A].delayed = true
	controller.read()
	controller.select_characteristic(Fakes.SERVICE_B, Fakes.CHARACTERISTIC)
	controller.select_device(B)
	manager.devices[A].complete(manager.devices[A].waiting.pop_front())
	manager.devices[A].delayed = false
	await tick()
	assert("A中" in controller.sessions[A].latest[key_a])
	assert(controller.current().address == B and controller.sessions[A].service_uuid == Fakes.SERVICE_B)
	controller.select_device(A)
	controller.select_characteristic(Fakes.SERVICE_A, Fakes.CHARACTERISTIC)
	manager.devices[A].reject = "subscribe"
	controller.toggle_subscription()
	await tick()
	assert(controller.current().subscriptions.is_empty() and controller.current().pending.is_empty())
	assert(not view.gatt.subscribe_button.disabled)
	manager.devices[A].reject = ""
	controller.toggle_subscription()
	assert(controller.current().subscriptions.is_empty()) # Changes only after success.
	await tick()
	assert(controller.current().subscriptions.has(key_a))
	controller.discover()
	await tick()
	assert(controller.current().subscriptions.has(key_a)) # Rediscovery does not unsubscribe the native device.
	manager.devices[A].reject = "unsubscribe"
	controller.toggle_subscription()
	await tick()
	assert(controller.current().subscriptions.has(key_a))
	manager.devices[A].reject = ""
	controller.toggle_subscription()
	await tick()
	assert(controller.current().subscriptions.is_empty())
	# Shared characteristic UUIDs stay isolated across devices AND services.
	manager.devices[A].notify(Fakes.SERVICE_A, PackedByteArray([0x11]))
	manager.devices[A].notify(Fakes.SERVICE_B, PackedByteArray([0x22]))
	manager.devices[B].notify(Fakes.SERVICE_A, PackedByteArray([0x33]))
	await tick()
	assert("HEX  11" in controller.sessions[A].latest[key_a])
	assert("HEX  22" in controller.sessions[A].latest[controller.current().key(Fakes.SERVICE_B, Fakes.CHARACTERISTIC)])
	assert("HEX  33" in controller.sessions[B].latest[controller.sessions[B].key(Fakes.SERVICE_A, Fakes.CHARACTERISTIC)])
	# Sync rejection releases controls, retaining the authoritative connection.
	for operation in ["read", "write", "discover_services", "disconnect"]:
		manager.devices[A].reject = operation
		match operation:
			"read": controller.read()
			"write": controller.write()
			"discover_services": controller.discover()
			"disconnect": controller.disconnect_device(A)
		await tick()
		assert(controller.current().pending.is_empty() and controller.current().connected)
		assert(not view.gatt.read_button.disabled)
	manager.devices[A].reject = ""
	controller.toggle_subscription()
	await tick()
	manager.devices[A].invalidate("NOTIFICATION_OVERFLOW")
	await tick()
	assert(controller.current().subscriptions.is_empty() and controller.current().connected)
	manager.devices[A].invalidate("SERVICES_CHANGED")
	await tick()
	assert(controller.current().services.is_empty() and not view.gatt.discover.disabled)
	controller.discover()
	await tick()
	assert(controller.current().services.size() == 2)
	manager.devices[A].lose_connection()
	await tick()
	assert(not controller.current().connected and controller.current().services.is_empty())
	assert(controller.sessions[B].connected and not view.devices.connect_button.disabled)
	manager.devices[A].reject = "connect"
	controller.connect_device(A)
	await tick()
	assert(controller.current().pending.is_empty() and not view.devices.connect_button.disabled)
	manager.devices[A].reject = ""
	controller.connect_device(A)
	await tick()
	assert(controller.current().connected)
	manager.lose_adapter()
	await tick()
	assert(not controller.ready_for_ble and not controller.sessions[B].connected)
	assert(not view.retry.disabled and view.retry.visible)
	manager.reject_initialize = true
	controller.retry_initialize()
	await tick()
	assert(not controller.initializing and not controller.ready_for_ble)
	manager.reject_initialize = false
	controller.retry_initialize()
	await tick()
	assert(controller.ready_for_ble)
	controller._permission_denied(PackedStringArray(["BLUETOOTH_CONNECT"]))
	await tick()
	assert(not controller.ready_for_ble and "权限" in view.status_label.text)
	controller.retry_initialize()
	await tick()
	controller.connect_device(A)
	controller.connect_device(B)
	await tick()
	controller.select_characteristic(Fakes.SERVICE_A, Fakes.CHARACTERISTIC)
	# Overflow stays bounded, even when a whole notification burst arrives in one frame.
	for index in range(2100):
		manager.devices[A].notify(Fakes.SERVICE_A, PackedByteArray([index % 256]))
	await tick()
	assert(controller.logs.entries.size() == 2000)
	controller._log(B, "另一台设备的日志")
	controller._log(A, "需处理的错误", true)
	view.logs.scope.select(1)
	view.logs.errors.button_pressed = true
	view.logs.refresh()
	assert("需处理的错误" in view.logs.visible_content() and not "另一台" in view.logs.visible_content())
	view.logs._clear()
	assert(controller.logs.visible(A, true).is_empty() and controller.logs.visible(B).size() == 1)
	assert(Codec.utf8(PackedByteArray([0xFF, 0xC0, 0x80, 0xED, 0xA0, 0x80])) == "������")
	assert(Codec.utf8("😀中文".to_utf8_buffer()) == "😀中文")
	assert(Codec.encode("01　AB\tCD\nEF", "HEX").data == PackedByteArray([1, 171, 205, 239]))
	# Use compact sample data in optional screenshots.
	controller.logs.entries.clear()
	controller._log(A, "读取成功\n" + Codec.describe("BLE 已就绪".to_utf8_buffer()))
	controller._log(B, "连接成功，服务发现完成")
	view.logs.scope.select(0)
	view.logs.errors.button_pressed = false
	await _layouts()
	view.queue_free()
	view = null
	controller.queue_free()
	await process_frame
	print("GDBLE debugger test passed.")
	quit(0)

func _native_and_permission_checks() -> void:
	var extension := "res://addons/gdble/gdble.gdextension"
	if not GDExtensionManager.is_extension_loaded(extension):
		assert(GDExtensionManager.load_extension(extension) == GDExtensionManager.LOAD_STATUS_OK)
	var device = ClassDB.instantiate("BleDevice")
	var port = DeviceAccess.new(device)
	var results: Array = []
	port.ble_event.connect(func(value): results.append(value))
	for operation in ["connect", "disconnect", "discover_services", "read", "write", "subscribe", "unsubscribe"]:
		var id: int = port.request(operation, Fakes.SERVICE_A, Fakes.CHARACTERISTIC, PackedByteArray([1]), true)
		assert(id > 0 and results.back().operation_id == id)
		assert(results.back().error.code == "NOT_INITIALIZED")
		assert(not port.connection_active())
	var gate_manager = Fakes.Manager.new()
	gate_manager.reject_initialize = true
	var gated = Controller.new()
	root.add_child(gated)
	gated.configure(gate_manager, true)
	await process_frame
	await process_frame
	assert(not gated.initializing and not gated.ready_for_ble)
	var first_gate = gated._gate
	gate_manager.reject_initialize = false
	gated.retry_initialize()
	assert(first_gate != gated._gate)
	await process_frame
	await process_frame
	assert(gated.ready_for_ble and gate_manager.calls.size() == 2)
	gated.queue_free()
	await process_frame

func _layouts() -> void:
	var arguments := OS.get_cmdline_user_args()
	var destination := ""
	if "--visual-dir" in arguments:
		destination = arguments[arguments.find("--visual-dir") + 1]
		DirAccess.make_dir_recursive_absolute(destination)
	for dimensions in [Vector2i(1280, 800), Vector2i(360, 800), Vector2i(800, 360)]:
		root.size = dimensions
		await tick()
		assert(view.narrow == (dimensions.x < 760 or dimensions.y < 600))
		assert(view.current_label.get_global_rect().end.x <= dimensions.x)
		assert(view.current_label.visible)
		for page in range(3) if view.narrow else [0]:
			if view.narrow: view.tabs.current_tab = page
			await tick()
			for panel in [view.devices, view.gatt, view.logs]:
				if panel.is_visible_in_tree():
					assert(panel.get_global_rect().end.x <= dimensions.x)
			assert(view.gatt.send.custom_minimum_size.y >= 40)
			if not destination.is_empty():
				await RenderingServer.frame_post_draw
				root.get_texture().get_image().save_png(destination.path_join("%dx%d-%d.png" % [dimensions.x, dimensions.y, page]))
		# Every operation remains reachable by scrolling, including short landscape.
		if view.narrow: view.tabs.current_tab = 1
		await tick()
		view.gatt.ensure_control_visible(view.gatt.send)
		await tick()
		assert(view.gatt.get_global_rect().encloses(view.gatt.send.get_global_rect()), "Send outside viewport at %s (view %s, minimum %s): %s / %s" % [dimensions, view.size, view.get_minimum_size(), view.gatt.get_global_rect(), view.gatt.send.get_global_rect()])
		if not destination.is_empty():
			await RenderingServer.frame_post_draw
			root.get_texture().get_image().save_png(destination.path_join("%dx%d-send.png" % [dimensions.x, dimensions.y]))
		view.gatt.scroll_vertical = 0
		if view.narrow:
			view.tabs.current_tab = 0
			await tick()
			view.devices.ensure_control_visible(view.devices.disconnect_button)
			await tick()
			assert(view.devices.get_global_rect().encloses(view.devices.disconnect_button.get_global_rect()))
	root.size = Vector2i(1280, 800)
	await tick()
	assert(not view.narrow)
	for panel in [view.devices, view.gatt, view.logs]:
		assert(panel.is_visible_in_tree() and panel.size.y > 0)
	root.size = Vector2i(360, 800)
	controller._permission_denied(PackedStringArray(["android.permission.BLUETOOTH_SCAN", "android.permission.BLUETOOTH_CONNECT"]))
	await tick()
	if not destination.is_empty():
		await RenderingServer.frame_post_draw
		root.get_texture().get_image().save_png(destination.path_join("360x800-permission.png"))
	assert(view.retry.is_visible_in_tree() and view.retry.get_global_rect().end.x <= 360, "Retry outside viewport: %s" % view.retry.get_global_rect())
	assert(view.status_label.get_global_rect().end.x <= 360)
