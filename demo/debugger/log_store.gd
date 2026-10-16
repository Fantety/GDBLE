extends RefCounted

const LIMIT := 2000
var entries: Array = []
var revision := 0

func add(address: String, message: String, is_error := false) -> void:
	revision += 1
	entries.append({"time": Time.get_datetime_string_from_system().replace("T", " "),
		"address": address, "message": message, "error": is_error})
	if entries.size() > LIMIT:
		entries.pop_front()

func visible(address := "", errors_only := false) -> Array:
	return entries.filter(func(entry): return (address.is_empty() or entry.address == address) and (not errors_only or entry.error))

func clear_scope(address := "", errors_only := false) -> void:
	revision += 1
	entries = entries.filter(func(entry): return not ((address.is_empty() or entry.address == address) and (not errors_only or entry.error)))

func content(address := "", errors_only := false) -> String:
	var lines := PackedStringArray()
	for entry in visible(address, errors_only):
		lines.append("%s  %s  %s\n%s" % [entry.time, "错误" if entry.error else "信息", entry.address if not entry.address.is_empty() else "适配器", entry.message])
	return "\n\n".join(lines)
