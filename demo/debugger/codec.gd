extends RefCounted
## Binary input and loss-tolerant display, shared by the UI and tests.

static func encode(value: String, format: String) -> Dictionary:
	if format == "UTF-8":
		return {"ok": true, "data": value.to_utf8_buffer(), "error": ""}
	var compact := ""
	for character in value:
		var code := character.unicode_at(0)
		if code in [9, 10, 11, 12, 13, 32, 0x85, 0xA0, 0x1680, 0x2028, 0x2029, 0x202F, 0x205F, 0x3000] or (code >= 0x2000 and code <= 0x200A):
			continue
		if not character.to_lower() in "0123456789abcdef":
			return {"ok": false, "data": PackedByteArray(), "error": "HEX 只能包含 0–9、A–F 和空白分隔符"}
		compact += character
	if compact.length() % 2:
		return {"ok": false, "data": PackedByteArray(), "error": "HEX 每个字节需要两位，请补全最后一个字节"}
	return {"ok": true, "data": compact.hex_decode(), "error": ""}

static func hex(data: PackedByteArray) -> String:
	var parts := PackedStringArray()
	for byte in data:
		parts.append("%02X" % byte)
	return " ".join(parts)

static func utf8(data: PackedByteArray) -> String:
	# Godot's UTF-8 conversion reports engine errors for arbitrary binary data.
	# Decode explicitly with replacement characters so any BLE payload is safe.
	var output := ""
	var index := 0
	while index < data.size():
		var first := int(data[index])
		var count := 1
		var code := first
		if first >= 0xC2 and first <= 0xDF:
			count = 2
			code = first & 0x1F
		elif first >= 0xE0 and first <= 0xEF:
			count = 3
			code = first & 0x0F
		elif first >= 0xF0 and first <= 0xF4:
			count = 4
			code = first & 0x07
		elif first >= 0x80:
			output += "�"
			index += 1
			continue
		var valid := index + count <= data.size()
		if valid:
			for offset in range(1, count):
				var next := int(data[index + offset])
				if next < 0x80 or next > 0xBF:
					valid = false
					break
				code = (code << 6) | (next & 0x3F)
		if (count == 2 and code < 0x80) or (count == 3 and code < 0x800) or (count == 4 and code < 0x10000):
			valid = false
		if code > 0x10FFFF or (code >= 0xD800 and code <= 0xDFFF):
			valid = false
		if valid:
			output += String.chr(code) if code != 0 else "␀"
			index += count
		else:
			output += "�"
			index += 1
	return output

static func describe(data: PackedByteArray) -> String:
	return "%d 字节\nHEX  %s\nUTF-8  %s" % [data.size(), hex(data), utf8(data)]
