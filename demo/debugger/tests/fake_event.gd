extends RefCounted

static func make(operation: String, phase: String, id: int, address := "", data = null, service := "", characteristic := "", code := "") -> Dictionary:
	return {"operation": operation, "phase": phase, "operation_id": id,
		"terminal": phase in ["succeeded", "failed", "cancelled"] and id != 0,
		"device_address": address, "service_uuid": service, "characteristic_uuid": characteristic,
		"data": data, "error": {"code": code, "message": "模拟拒绝", "retryable": true} if not code.is_empty() else {}}
