extends Control
## Set controller before entering the tree to inject a test manager.

const Controller = preload("res://debugger/controller.gd")
const UI = preload("res://debugger/ui.gd")
const DevicePanel = preload("res://debugger/device_panel.gd")
const GattPanel = preload("res://debugger/gatt_panel.gd")
const LogPanel = preload("res://debugger/log_panel.gd")

var controller
var devices
var gatt
var logs
var tabs: TabContainer
var desktop: VSplitContainer
var columns: HSplitContainer
var current_label: Label
var status_label: Label
var retry: Button
var narrow := false
var _dirty := true

func _ready() -> void:
	if OS.get_name() == "Android":
		# Keep controls close to density-independent pixels on high-DPI phones.
		get_window().content_scale_factor = maxf(1.0, DisplayServer.screen_get_dpi() / 160.0)
	theme = UI.theme()
	var background := ColorRect.new()
	background.color = Color("111d29")
	background.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	background.mouse_filter = Control.MOUSE_FILTER_IGNORE
	add_child(background)
	var margin := MarginContainer.new()
	margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 12)
	add_child(margin)
	var body = UI.column()
	margin.add_child(body)
	var heading = UI.row()
	body.add_child(heading)
	var title = UI.label("GDBLE 调试工具", 22)
	title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	heading.add_child(title)
	retry = UI.button("重试初始化", func(): controller.retry_initialize())
	heading.add_child(retry)
	status_label = UI.label("")
	body.add_child(status_label)
	current_label = UI.label("")
	current_label.add_theme_color_override("font_color", Color("80d7e8"))
	body.add_child(current_label)
	if controller == null:
		controller = Controller.new()
		add_child(controller)
		controller.configure()
	controller.changed.connect(func(_kind, _address): _dirty = true)
	desktop = VSplitContainer.new()
	desktop.size_flags_vertical = Control.SIZE_EXPAND_FILL
	desktop.split_offset = 460
	body.add_child(desktop)
	columns = HSplitContainer.new()
	columns.split_offset = 340
	desktop.add_child(columns)
	tabs = TabContainer.new()
	tabs.get_tab_bar().custom_minimum_size.y = 44
	tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	tabs.visible = false
	tabs.tab_changed.connect(func(_index): _dirty = true)
	body.add_child(tabs)
	devices = DevicePanel.new()
	gatt = GattPanel.new()
	logs = LogPanel.new()
	for panel in [devices, gatt, logs]:
		panel.setup(controller)
	columns.add_child(devices)
	columns.add_child(gatt)
	desktop.add_child(logs)
	devices.custom_minimum_size.x = 285
	resized.connect(_layout)
	_layout()
	var timer := Timer.new()
	timer.wait_time = 0.1
	timer.timeout.connect(func():
		if _dirty:
			refresh())
	add_child(timer)
	timer.start()
	refresh()

func _layout() -> void:
	var use_tabs := size.x < 760 or size.y < 600
	if use_tabs != narrow:
		narrow = use_tabs
		if narrow:
			devices.custom_minimum_size.x = 0
			for panel in [devices, gatt, logs]:
				panel.reparent(tabs)
		else:
			devices.reparent(columns)
			gatt.reparent(columns)
			logs.reparent(desktop)
			# TabContainer hides inactive pages; reparenting preserves that flag.
			for panel in [devices, gatt, logs]:
				panel.show()
			devices.custom_minimum_size.x = 285
	desktop.visible = not narrow
	tabs.visible = narrow

func refresh() -> void:
	_dirty = false
	status_label.text = controller.status
	retry.disabled = controller.initializing
	retry.visible = not controller.ready_for_ble
	var session = controller.current()
	current_label.text = "当前设备：%s · %s · %s" % [session.name, session.address, session.state] if session else "当前设备：未选择"
	devices.refresh()
	gatt.refresh()
	logs.refresh()
