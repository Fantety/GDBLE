extends RefCounted
## A deliberately synchronous fake also exercises signals emitted before return.

const SERVICE_A := "0000180f-0000-1000-8000-00805f9b34fb"
const SERVICE_B := "0000180a-0000-1000-8000-00805f9b34fb"
const CHARACTERISTIC := "00002a19-0000-1000-8000-00805f9b34fb"
const Events = preload("res://debugger/tests/fake_event.gd")

class Manager extends RefCounted:
	signal ble_event(value: Dictionary)
	var devices := {}
	var calls: Array = []
	var initialized := false
	var counter := 0
	var scan_id := 0
	var reject_initialize := false
	var reject_scan := false
	func next_id() -> int:
		counter += 1
		return counter
	func is_initialized() -> bool:
		return initialized
	func initialize() -> int:
		var id := next_id()
		initialized = not reject_initialize
		calls.append({"operation": "initialize", "id": id})
		ble_event.emit(Events.make("initialize", "succeeded" if initialized else "failed", id, "", {}, "", "", "" if initialized else "ADAPTER_NOT_AVAILABLE"))
		return id
	func start_scan(seconds: float) -> int:
		scan_id = next_id()
		calls.append({"operation": "scan", "seconds": seconds})
		ble_event.emit(Events.make("scan", "failed" if reject_scan else "started", scan_id, "", null, "", "", "REJECTED" if reject_scan else ""))
		return scan_id
	func stop_scan() -> int:
		ble_event.emit(Events.make("scan", "succeeded", scan_id))
		return scan_id
	func advertise(address: String, name: String, rssi: int) -> void:
		ble_event.emit(Events.make("scan", "progress", scan_id, address, {"kind": "updated", "address": address, "name": name, "rssi": rssi}))
	func get_or_create_device(address: String):
		if not devices.has(address):
			var device := Device.new()
			device.address = address
			device.manager_ref = weakref(self)
			devices[address] = device
		return devices[address]
	func lose_adapter() -> void:
		initialized = false
		for device in devices.values():
			device.connected = false
		ble_event.emit(Events.make("initialize", "failed", 0, "", null, "", "", "ADAPTER_NOT_AVAILABLE"))

class Device extends RefCounted:
	signal ble_event(value: Dictionary)
	var manager_ref: WeakRef
	var address := ""
	var connected := false
	var reject := ""
	var delayed := false
	var waiting: Array = []
	var calls: Array = []
	var services := [
		{"uuid": SERVICE_A, "characteristics": [{"uuid": CHARACTERISTIC, "properties": {"read": true, "write": true, "write_without_response": true, "notify": true}}]},
		{"uuid": SERVICE_B, "characteristics": [{"uuid": CHARACTERISTIC, "properties": {"write_without_response": true, "indicate": true}}]}
	]
	func connection_active() -> bool:
		return connected
	func request(operation: String, service := "", characteristic := "", data = null, response := true) -> int:
		var id: int = manager_ref.get_ref().next_id()
		var request := {"operation": operation, "id": id, "service": service, "characteristic": characteristic, "data": data, "response": response}
		calls.append(request)
		if delayed:
			waiting.append(request)
		else:
			complete(request)
		return id
	func complete(request: Dictionary) -> void:
		var operation: String = request.operation
		var failure := reject == operation
		var data = request.data
		if not failure:
			if operation == "connect": connected = true
			if operation == "disconnect": connected = false
			if operation == "discover_services": data = services
			if operation == "read": data = PackedByteArray([0x41, 0xE4, 0xB8, 0xAD])
		var value := Events.make(operation, "failed" if failure else "succeeded", request.id, address, data, request.service, request.characteristic, "REJECTED" if failure else "")
		ble_event.emit(value)
		manager_ref.get_ref().ble_event.emit(value) # Native manager forwards the same event.
	func notify(service: String, data: PackedByteArray) -> void:
		var value := Events.make("notification", "received", 0, address, data, service, CHARACTERISTIC)
		ble_event.emit(value)
		manager_ref.get_ref().ble_event.emit(value)
	func invalidate(code: String) -> void:
		var value := Events.make("notification", "failed", 0, address, null, "", "", code)
		ble_event.emit(value)
		manager_ref.get_ref().ble_event.emit(value)
	func lose_connection() -> void:
		connected = false
		ble_event.emit(Events.make("disconnect", "succeeded", 0, address))
