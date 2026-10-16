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
