extends SceneTree

const EXTENSION_PATH := "res://addons/gdble/gdble.gdextension"

var events: Array[Dictionary] = []

func _initialize() -> void:
	create_timer(10.0).timeout.connect(_on_timeout)
	# A fresh checkout has no editor-generated .godot/extension_list.cfg.
	if not GDExtensionManager.is_extension_loaded(EXTENSION_PATH):
		var status := GDExtensionManager.load_extension(EXTENSION_PATH)
		if status != GDExtensionManager.LOAD_STATUS_OK:
			push_error("Failed to load GDBLE extension (status %s)" % status)
			quit(1)
			return

	assert(ClassDB.class_exists(&"BluetoothManager"))
	assert(ClassDB.class_exists(&"BleDevice"))

	# Resolve extension classes only after loading, not while parsing this script.
	var manager = ClassDB.instantiate(&"BluetoothManager")
	assert(not manager.has_method(&"test_use_fake_backend"))
	root.add_child(manager)
	assert(manager.has_signal(&"ble_event"))
	manager.ble_event.connect(func(event: Dictionary): events.append(event))

	var operation_id: int = manager.start_scan(-1.0)
	assert(operation_id > 0)
	while events.is_empty():
		await process_frame
	await process_frame

	assert(events.size() == 1)
	var event := events[0]
	for field in [
		&"operation", &"phase", &"operation_id", &"terminal",
		&"device_address", &"service_uuid", &"characteristic_uuid",
		&"data", &"error"
	]:
		assert(event.has(field))
	assert(event.operation == &"scan")
	assert(event.phase == &"failed")
	assert(event.operation_id == operation_id)
	assert(event.terminal)
	assert(event.error.code == "INVALID_ARGUMENT")
	# Local rejections must not be lost when a frame submits more than 1024 calls.
	events.clear()
	var submitted: Dictionary = {}
	for index in range(1500):
		var rejected_id: int = manager.start_scan(-1.0)
		assert(not submitted.has(rejected_id))
		submitted[rejected_id] = true
	assert(events.size() == 1500)
	for rejected in events:
		assert(rejected.terminal)
		assert(submitted.erase(rejected.operation_id))
	assert(submitted.is_empty())
	for invalid_timeout in [INF, NAN, 1e100]:
		var before := events.size()
		manager.start_scan(invalid_timeout)
		assert(events.size() == before + 1)
		assert(events.back().error.code == "INVALID_ARGUMENT")

	var standalone = ClassDB.instantiate(&"BleDevice")
	var device_events: Array[Dictionary] = []
	assert(standalone.has_signal(&"ble_event"))
	standalone.ble_event.connect(func(device_event: Dictionary): device_events.append(device_event))
	var connect_id: int = standalone.connect_async()
	assert(connect_id > 0)
	assert(device_events.size() == 1)
	assert(device_events[0].operation == &"connect")
	assert(device_events[0].error.code == "NOT_INITIALIZED")
	var callback_states: Array[bool] = []
	var state_listener := func(_event: Dictionary): callback_states.append(standalone.is_connected())
	standalone.ble_event.connect(state_listener)
	for method in [&"disconnect", &"connect_async", &"discover_services"]:
		var before := device_events.size()
		var rejected_id: int = standalone.call(method)
		assert(rejected_id > 0)
		assert(device_events.size() == before + 1)
		assert(device_events.back().error.code == "NOT_INITIALIZED")
		assert(not standalone.is_connected())
		assert(not callback_states.back())
	standalone.ble_event.disconnect(state_listener)

	manager.queue_free()
	await process_frame
	print("GDBLE smoke test passed.")
	quit(0)

func _on_timeout() -> void:
	push_error("GDBLE smoke test timed out before completing all checks")
	quit(1)
