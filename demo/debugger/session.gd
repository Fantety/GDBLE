extends RefCounted
## All mutable state belongs to one normalized device identity.

var address := ""
var name := "未命名设备"
var rssi = null
var device
var state := "未连接"
var connected := false
var services: Array = []
var service_uuid := ""
var characteristic_uuid := ""
var draft := ""
var format := "HEX"
var with_response := true
var latest := {}
var subscriptions := {}
var pending := {}

static func normalize(value: String) -> String:
	return value.strip_edges().to_upper()

func key(service := "", characteristic := "") -> String:
	return address + "|" + service.to_lower() + "|" + characteristic.to_lower()

func properties() -> Dictionary:
	for service in services:
		if str(service.get("uuid", "")).to_lower() != service_uuid:
			continue
		for characteristic in service.get("characteristics", []):
			if str(characteristic.get("uuid", "")).to_lower() == characteristic_uuid:
				return characteristic.get("properties", {})
	return {}

func select(service: String, characteristic: String) -> void:
	service_uuid = service.to_lower()
	characteristic_uuid = characteristic.to_lower()
	var props := properties()
	with_response = bool(props.get("write", false))

func invalidate_gatt() -> void:
	services.clear()
	subscriptions.clear()
	service_uuid = ""
	characteristic_uuid = ""
	for operation in pending.keys():
		if operation not in ["connect", "disconnect"]:
			pending.erase(operation)

func disconnected() -> void:
	connected = false
	state = "未连接"
	invalidate_gatt()
	pending.clear()
