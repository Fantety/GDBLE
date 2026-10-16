extends ScrollContainer

const UI = preload("res://debugger/ui.gd")
var controller
var filter: LineEdit
var devices: Tree
var duration: SpinBox
var scan: Button
var connect_button: Button
var disconnect_button: Button
var count: Label
var _syncing := false
var _rows := {}
var _body: VBoxContainer

func setup(value) -> void:
	controller = value
	name = "设备"
	horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	_body = UI.column()
	_body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	add_child(_body)
	_body.add_child(UI.label("附近设备", 20))
	var scan_row = UI.row()
	_body.add_child(scan_row)
	scan = UI.button("开始扫描", _scan)
	scan.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scan_row.add_child(scan)
	duration = SpinBox.new()
	duration.min_value = 1
	duration.max_value = 300
	duration.value = 10
	duration.suffix = "秒"
	duration.custom_minimum_size = Vector2(115, 40)
	scan_row.add_child(duration)
	filter = LineEdit.new()
	filter.placeholder_text = "筛选名称或地址"
	filter.custom_minimum_size.y = 40
	filter.text_changed.connect(func(_text): refresh())
	_body.add_child(filter)
	count = UI.label("")
	_body.add_child(count)
	devices = Tree.new()
	devices.hide_root = true
	devices.size_flags_vertical = Control.SIZE_EXPAND_FILL
	devices.custom_minimum_size.y = 100
	devices.item_selected.connect(_selected)
	devices.create_item()
	_body.add_child(devices)
	var actions = UI.row()
	_body.add_child(actions)
	connect_button = UI.button("连接设备", func(): controller.connect_device(controller.selected_address))
	disconnect_button = UI.button("断开连接", func(): controller.disconnect_device(controller.selected_address))
	for button in [connect_button, disconnect_button]:
		button.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		actions.add_child(button)

func _scan() -> void:
	if controller.scanning:
		controller.stop_scan()
	else:
		controller.start_scan(duration.value)

func _selected() -> void:
	if not _syncing and devices.get_selected():
		controller.select_device(devices.get_selected().get_metadata(0))

func refresh() -> void:
	_syncing = true
	var visible_count := 0
	for session in controller.sessions.values():
		if not _rows.has(session.address):
			_rows[session.address] = devices.create_item(devices.get_root())
		var item: TreeItem = _rows[session.address]
		item.visible = filter.text.is_empty() or filter.text.to_lower() in (session.name + " " + session.address).to_lower()
		if item.visible:
			visible_count += 1
		var rssi: String = "%s dBm" % session.rssi if session.rssi != null else "RSSI 未知"
		item.set_text(0, "%s\n%s\n%s  ·  %s" % [session.name, session.address, rssi, session.state])
		item.set_metadata(0, session.address)
		item.set_tooltip_text(0, item.get_text(0))
		item.set_custom_minimum_height(78)
		if session.address == controller.selected_address:
			item.select(0)
	count.text = "%d 台设备 · 已连接 %d 台" % [visible_count, controller.sessions.values().filter(func(s): return s.connected).size()]
	if controller.sessions.is_empty():
		count.text = "正在扫描，等待附近设备广播…" if controller.scanning else "点击扫描查找附近的 BLE 设备"
	scan.text = "停止扫描" if controller.scanning else "开始扫描"
	if controller.scan_pending:
		scan.text = "正在停止…" if controller.scanning else "扫描启动中…"
	scan.disabled = not controller.ready_for_ble or controller.scan_pending
	duration.editable = not controller.scanning and not controller.scan_pending
	var current = controller.current()
	connect_button.disabled = not controller.ready_for_ble or current == null or current.connected or not current.pending.is_empty()
	disconnect_button.disabled = current == null or not current.connected or current.pending.has("disconnect")
	_syncing = false
