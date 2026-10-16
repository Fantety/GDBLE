extends ScrollContainer

const UI = preload("res://debugger/ui.gd")
var controller
var scope: OptionButton
var errors: CheckBox
var output: TextEdit
var count: Label
var _rendered := ""

func setup(value) -> void:
	controller = value
	name = "日志"
	horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	var body = UI.column()
	body.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	body.size_flags_vertical = Control.SIZE_EXPAND_FILL
	add_child(body)
	var filters = UI.row()
	body.add_child(filters)
	scope = OptionButton.new()
	scope.add_item("全部设备")
	scope.add_item("当前设备")
	scope.custom_minimum_size.y = 40
	scope.item_selected.connect(func(_index): refresh())
	filters.add_child(scope)
	errors = CheckBox.new()
	errors.text = "仅错误"
	errors.toggled.connect(func(_on): refresh())
	filters.add_child(errors)
	var actions = UI.row()
	body.add_child(actions)
	count = UI.label("")
	count.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	actions.add_child(count)
	actions.add_child(UI.button("复制", func(): DisplayServer.clipboard_set(visible_content())))
	actions.add_child(UI.button("清空", _clear))
	output = TextEdit.new()
	output.editable = false
	output.wrap_mode = TextEdit.LINE_WRAPPING_BOUNDARY
	output.size_flags_vertical = Control.SIZE_EXPAND_FILL
	output.custom_minimum_size.y = 100
	body.add_child(output)

func _scope_address() -> String:
	return controller.selected_address if scope.selected == 1 else ""

func visible_content() -> String:
	if scope.selected == 1 and controller.selected_address.is_empty():
		return ""
	return controller.logs.content(_scope_address(), errors.button_pressed)

func _clear() -> void:
	if scope.selected == 1 and controller.selected_address.is_empty():
		return
	controller.logs.clear_scope(_scope_address(), errors.button_pressed)
	refresh()

func refresh() -> void:
	if not is_visible_in_tree():
		return
	var version := "%d|%d|%s|%s" % [controller.logs.revision, scope.selected, _scope_address(), errors.button_pressed]
	if version == _rendered:
		return
	_rendered = version
	var scrollbar := output.get_v_scroll_bar()
	var at_end := scrollbar.value >= scrollbar.max_value - scrollbar.page - 2
	var position := output.scroll_vertical
	output.text = visible_content()
	if at_end:
		output.set_deferred("scroll_vertical", float(output.get_line_count()))
	else:
		output.scroll_vertical = position
	count.text = "%d / 2000 条" % controller.logs.entries.size()
