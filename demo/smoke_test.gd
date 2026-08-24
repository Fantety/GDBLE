extends SceneTree

var events: Array[Dictionary] = []

func _initialize() -> void:
	assert(ClassDB.class_exists(&"BluetoothManager"))
	assert(ClassDB.class_exists(&"BleDevice"))

	var manager := BluetoothManager.new()
	root.add_child(manager)
	assert(manager.has_signal(&"ble_event"))
	manager.ble_event.connect(func(event: Dictionary): events.append(event))

	var operation_id: int = manager.start_scan(-1.0)
	assert(operation_id > 0)
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

	var standalone := BleDevice.new()
	var device_events: Array[Dictionary] = []
	assert(standalone.has_signal(&"ble_event"))
	standalone.ble_event.connect(func(device_event: Dictionary): device_events.append(device_event))
	var connect_id: int = standalone.connect_async()
	assert(connect_id > 0)
	assert(device_events.size() == 1)
	assert(device_events[0].operation == &"connect")
	assert(device_events[0].error.code == "NOT_INITIALIZED")

	manager.queue_free()
	await process_frame
	quit(0)
