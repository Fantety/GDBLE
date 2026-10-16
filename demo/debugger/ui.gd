extends RefCounted

const GAP := 10

static func label(value: String, size := 16) -> Label:
	var node := Label.new()
	node.text = value
	node.add_theme_font_size_override("font_size", size)
	node.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	return node

static func button(value: String, action: Callable) -> Button:
	var node := Button.new()
	node.text = value
	node.custom_minimum_size.y = 40
	node.pressed.connect(action)
	return node

static func row() -> HBoxContainer:
	var node := HBoxContainer.new()
	node.add_theme_constant_override("separation", GAP)
	return node

static func column() -> VBoxContainer:
	var node := VBoxContainer.new()
	node.add_theme_constant_override("separation", GAP)
	return node

static func theme() -> Theme:
	var result := Theme.new()
	result.default_font_size = 16
	var font := SystemFont.new()
	font.font_names = PackedStringArray(["Microsoft YaHei", "Noto Sans CJK SC", "Noto Sans", "sans-serif"])
	result.default_font = font
	result.set_color("font_color", "Label", Color("dde6ed"))
	result.set_color("font_color", "Button", Color("eef5fa"))
	result.set_color("font_disabled_color", "Button", Color("71808e"))
	result.set_color("font_readonly_color", "TextEdit", Color("dde6ed"))
	result.set_color("font_color", "Tree", Color("dde6ed"))
	for type in ["Button", "LineEdit", "TextEdit", "Tree", "TabContainer", "OptionButton", "SpinBox"]:
		var style := StyleBoxFlat.new()
		style.bg_color = Color("202d3b")
		style.border_color = Color("3b4f62")
		style.set_border_width_all(1)
		style.set_corner_radius_all(5)
		style.content_margin_left = 10
		style.content_margin_right = 10
		style.content_margin_top = 7
		style.content_margin_bottom = 7
		result.set_stylebox("normal" if type in ["Button", "LineEdit", "OptionButton"] else "panel", type, style)
		if type in ["Button", "OptionButton"]:
			var hover := style.duplicate()
			hover.bg_color = Color("304a61")
			result.set_stylebox("hover", type, hover)
			var pressed := style.duplicate()
			pressed.bg_color = Color("215a72")
			result.set_stylebox("pressed", type, pressed)
			var disabled := style.duplicate()
			disabled.bg_color = Color("19232e")
			result.set_stylebox("disabled", type, disabled)
	return result
