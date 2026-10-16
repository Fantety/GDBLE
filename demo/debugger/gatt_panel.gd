extends ScrollContainer

const UI = preload("res://debugger/ui.gd")
const Codec = preload("res://debugger/codec.gd")
var controller
var tree: Tree
var selection: RichTextLabel
var discover: Button
var read_button: Button
var subscribe_button: Button
var input: TextEdit
var format: OptionButton
var mode: OptionButton
var send: Button
var validation: Label
var latest: TextEdit
var _syncing := false
var _tree_version := ""
var _address := ""

func setup(value) -> void:
	controller = value
	name = "调试"
	horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	var body = UI.column()
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	add_child(body)
	var heading = UI.row()
	body.add_child(heading)
	var title = UI.label("GATT 服务与特征", 20)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	heading.add_child(title)
	discover = UI.button("重新发现", func(): controller.discover())
	heading.add_child(discover)
	tree = Tree.new()
	tree.hide_root = true
	tree.set_column_custom_minimum_width(0, 530)
	tree.custom_minimum_size.y = 190
	tree.item_selected.connect(_selected)
	body.add_child(tree)
	selection = RichTextLabel.new()
	selection.fit_content = true
	selection.selection_enabled = true
	selection.custom_minimum_size.y = 58
	body.add_child(selection)
	var actions = UI.row()
	body.add_child(actions)
	read_button = UI.button("读取", func(): controller.read())
	subscribe_button = UI.button("订阅通知", func(): controller.toggle_subscription())
	for button in [read_button, subscribe_button]:
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		actions.add_child(button)
	body.add_child(UI.label("发送数据"))
	var options = UI.row()
	body.add_child(options)
	format = OptionButton.new()
	format.add_item("HEX")
	format.add_item("UTF-8")
	format.custom_minimum_size.y = 40
	format.item_selected.connect(_format_changed)
	options.add_child(format)
	mode = OptionButton.new()
	mode.add_item("有响应写入")
	mode.add_item("无响应写入")
	mode.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	mode.item_selected.connect(_mode_changed)
	options.add_child(mode)
	input = TextEdit.new()
	input.placeholder_text = "输入 HEX（如 01 AF 20）或 UTF-8 文本"
	input.wrap_mode = TextEdit.LINE_WRAPPING_BOUNDARY
	input.custom_minimum_size.y = 84
	input.text_changed.connect(_input_changed)
	body.add_child(input)
	validation = UI.label("")
	validation.add_theme_color_override("font_color", Color("ffbf91"))
	body.add_child(validation)
	send = UI.button("发送", _send)
	body.add_child(send)
	body.add_child(UI.label("当前特征 · 最近收发"))
	latest = _display(135)
	body.add_child(latest)

func _display(height: float) -> TextEdit:
	var result := TextEdit.new()
	result.editable = false
	result.wrap_mode = TextEdit.LINE_WRAPPING_BOUNDARY
	result.custom_minimum_size.y = height
	return result

func _selected() -> void:
	if _syncing or not tree.get_selected():
		return
	var identity = tree.get_selected().get_metadata(0)
	if identity is Array:
		controller.select_characteristic(identity[0], identity[1])

func _input_changed() -> void:
	if not _syncing and controller.current():
		controller.current().draft = input.text
		_validate()

func _format_changed(index: int) -> void:
	if not _syncing and controller.current():
		controller.current().format = format.get_item_text(index)
		_validate()

func _mode_changed(index: int) -> void:
	if not _syncing and controller.current():
		controller.current().with_response = index == 0

func _send() -> void:
	var result: Dictionary = controller.write()
	if not result.ok:
		validation.text = result.error

func _validate() -> void:
	var session = controller.current()
	var result := Codec.encode(input.text, session.format if session else "HEX")
	validation.text = result.error if not result.ok else "%d 字节" % result.data.size()
	var props: Dictionary = session.properties() if session else {}
	send.disabled = not controller._available(session) or not result.ok or not (props.get("write", false) or props.get("write_without_response", false))

func refresh() -> void:
	_syncing = true
	var session = controller.current()
	var version: String = session.address + str(hash(session.services)) if session else ""
	if version != _tree_version:
		_tree_version = version
		tree.clear()
		var root := tree.create_item()
		if session:
			for service in session.services:
				var parent := tree.create_item(root)
				parent.set_text(0, "服务  " + str(service.uuid))
				parent.set_selectable(0, false)
				parent.set_tooltip_text(0, str(service.uuid))
				for characteristic in service.get("characteristics", []):
					var item := tree.create_item(parent)
					var props: Dictionary = characteristic.get("properties", {})
					var flags := PackedStringArray()
					for property in ["read", "write", "write_without_response", "notify", "indicate"]:
						if props.get(property, false):
							flags.append({"read": "读", "write": "写", "write_without_response": "无响应写", "notify": "通知", "indicate": "指示"}[property])
					item.set_text(0, "%s\n%s" % [characteristic.uuid, " · ".join(flags)])
					item.set_custom_minimum_height(52)
					item.set_tooltip_text(0, item.get_text(0))
					item.set_metadata(0, [str(service.uuid), str(characteristic.uuid)])
					if str(service.uuid).to_lower() == session.service_uuid and str(characteristic.uuid).to_lower() == session.characteristic_uuid:
						item.select(0)
	if _address != controller.selected_address:
		_address = controller.selected_address
		input.text = session.draft if session else ""
		format.select(0 if session == null or session.format == "HEX" else 1)
	var available: bool = controller._available(session)
	var props: Dictionary = session.properties() if session else {}
	discover.disabled = not available
	read_button.disabled = not available or not props.get("read", false)
	var subscribed: bool = session != null and session.subscriptions.has(session.key(session.service_uuid, session.characteristic_uuid))
	subscribe_button.text = "取消订阅" if subscribed else "订阅通知"
	subscribe_button.disabled = not available or not (props.get("notify", false) or props.get("indicate", false))
	mode.set_item_disabled(0, not props.get("write", false))
	mode.set_item_disabled(1, not props.get("write_without_response", false))
	mode.select(0 if session == null or session.with_response else 1)
	mode.disabled = not available or not (props.get("write", false) or props.get("write_without_response", false))
	input.editable = session != null
	format.disabled = session == null
	if session == null:
		selection.text = "请扫描并选择设备，再连接以发现服务"
	elif not session.connected:
		selection.text = "设备尚未连接，请在设备列表连接"
	elif session.characteristic_uuid.is_empty():
		selection.text = "请选择一个特征；服务失效时可点击重新发现"
	else:
		selection.text = "服务  %s\n特征  %s" % [session.service_uuid, session.characteristic_uuid]
	latest.text = session.latest.get(session.key(session.service_uuid, session.characteristic_uuid), "尚无数据；读取、发送或订阅后显示") if session else "尚无数据"
	_validate()
	_syncing = false
