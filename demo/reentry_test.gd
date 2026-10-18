extends SceneTree

var manager
var device
var events: Array[Dictionary] = []
var callbacks := 0
var forwarded := 0

func _initialize() -> void:
	create_timer(15.0).timeout.connect(func(): push_error("Reentry test timeout"); quit(1))
	var extension := "res://addons/gdble/gdble.gdextension"
	if not GDExtensionManager.is_extension_loaded(extension):
		assert(GDExtensionManager.load_extension(extension) == GDExtensionManager.LOAD_STATUS_OK)
	manager = ClassDB.instantiate(&"BluetoothManager")
	assert(manager.has_method(&"test_use_fake_backend"))
	manager.test_use_fake_backend()
	root.add_child(manager)
	manager.ble_event.connect(func(e: Dictionary): events.append(e))
	manager.initialize()
	while not manager.is_initialized():
		await process_frame
	await _test_scan_cancellation()
	await _test_deferred_requests_and_reason_payloads()
	device = manager.get_or_create_device("aa:bb:cc:dd:ee:ff")
	device.connected.connect(_device_connected)
	manager.device_connected.connect(func(address: String):
		if address == "aa:bb:cc:dd:ee:ff":
			assert(callbacks == 1) # Device callbacks precede Manager forwarding.
			forwarded += 1)
	device.connect_async()
	while forwarded == 0:
		await process_frame
	while is_instance_valid(manager):
		await process_frame
	var retained: Array[Dictionary] = []
	device.ble_event.connect(func(e: Dictionary): retained.append(e))
	device.connect_async()
	assert(retained.size() == 1)
	assert(retained[0].terminal)
	assert(retained[0].error.code == "CANCELLED") # Retains its closed client and ID sequence.
	print("GDBLE reentry test passed.")
	quit(0)

func _test_scan_cancellation() -> void:
	assert(manager.stop_scan() == 0)
	var old: int = manager.start_scan(0.01)
	# Keep Godot from draining old scan events while the BLE worker finishes.
	var deadline := Time.get_ticks_msec() + 2000
	while manager.stop_scan() != 0:
		assert(Time.get_ticks_msec() < deadline)
		OS.delay_msec(1)
	var next: int = manager.start_scan(0.0)
	assert(next != old)
	assert(manager.stop_scan() == next)
	var terminals := {}
	while terminals.size() < 2:
		await process_frame
		terminals.clear()
		for event in events:
			if event.operation_id in [old, next] and event.terminal:
				assert(not terminals.has(event.operation_id))
				terminals[event.operation_id] = true
	assert(manager.stop_scan() == 0)

func _test_deferred_requests_and_reason_payloads() -> void:
	var probe = manager.get_or_create_device("11:22:33:44:55:66")
	var pending := {}
	var completed := {}
	var listener := func(event: Dictionary):
		if event.terminal:
			assert(pending.has(event.operation_id))
			pending.erase(event.operation_id)
			completed[event.operation_id] = event
	probe.ble_event.connect(listener, CONNECT_DEFERRED)
	var id: int = probe.read_characteristic("invalid", "invalid")
	pending[id] = true
	assert(completed.is_empty())
	while not completed.has(id):
		await process_frame
	assert(completed[id].error.code == "INVALID_UUID")
	id = probe.connect_async()
	pending[id] = true
	while not completed.has(id):
		await process_frame
	assert(completed[id].phase == &"succeeded")
	id = probe.unsubscribe_characteristic("0000180d-0000-1000-8000-00805f9b34fb", "00002a37-0000-1000-8000-00805f9b34fb")
	pending[id] = true
	while not completed.has(id):
		await process_frame
	assert(completed[id].phase == &"succeeded")
	assert(completed[id].data is String)
	assert(completed[id].data == "already_unsubscribed")
	id = probe.disconnect()
	pending[id] = true
	while not completed.has(id):
		await process_frame
	assert(completed[id].phase == &"succeeded")
	assert(pending.is_empty())
	probe.ble_event.disconnect(listener)

func _device_connected() -> void:
	callbacks += 1
	assert(manager.is_initialized())
	assert(not manager.get_adapter_info().is_empty())
	assert(manager.get_device("aa:bb:cc:dd:ee:ff") == device)
	assert(manager.get_connected_devices().has(device))
	assert(device.is_connected())
	device.discover_services()
	manager.start_scan(-1.0) # Synchronous rejection reenters Manager dispatch.
	var other = manager.get_or_create_device("11:22:33:44:55:66")
	other.connect_async()
	manager.disconnect_device("11:22:33:44:55:66")
	manager.queue_free()
