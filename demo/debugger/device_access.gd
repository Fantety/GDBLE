extends RefCounted
## Narrow device port. Fakes implement the same port without native Object method
## name collisions (BleDevice overrides Object.disconnect/is_connected).

signal ble_event(event: Dictionary)
var native_device

func _init(value) -> void:
	native_device = value
	native_device.ble_event.connect(_forward)

func _forward(event: Dictionary) -> void:
	ble_event.emit(event)

func connection_active() -> bool:
	return native_device.is_connected()

func request(operation: String, service: String, characteristic: String, data: PackedByteArray, response: bool) -> int:
	match operation:
		"connect": return native_device.connect_async()
		"disconnect": return native_device.disconnect()
		"discover_services": return native_device.discover_services()
		"read": return native_device.read_characteristic(service, characteristic)
		"write": return native_device.write_characteristic(service, characteristic, data, response)
		"subscribe": return native_device.subscribe_characteristic(service, characteristic)
		"unsubscribe": return native_device.unsubscribe_characteristic(service, characteristic)
	return 0
