extends Node
## Routes adapter events once and device events to their owning session.

signal changed(kind: String, address: String)

const Session = preload("res://debugger/session.gd")
const DeviceAccess = preload("res://debugger/device_access.gd")
const Codec = preload("res://debugger/codec.gd")
const LogStore = preload("res://debugger/log_store.gd")
const PermissionStartup = preload("res://addons/gdble/permission_startup.gd")
const DEVICE_OPERATIONS := ["connect", "disconnect", "discover_services", "read", "write", "subscribe", "unsubscribe", "notification"]
const OPERATION_LABELS := {"connect": "连接", "disconnect": "断开连接", "discover_services": "发现服务", "read": "读取", "write": "写入", "subscribe": "订阅", "unsubscribe": "取消订阅", "notification": "通知"}

var manager
var sessions := {}
var selected_address := ""
var logs = LogStore.new()
var ready_for_ble := false
var initializing := false
var scanning := false
var scan_pending := false
var scan_id := 0
var status := "尚未初始化"
var _gate
var _use_permissions := true
var _events: Array = []
var _scheduled := false
var _device_factory: Callable

func configure(injected_manager = null, use_permissions := true, device_factory := Callable()) -> void:
	_use_permissions = use_permissions
	_device_factory = device_factory
	manager = injected_manager
	# A direct launch from a fresh checkout may not have an editor extension cache.
	if manager == null and not ClassDB.class_exists("BluetoothManager"):
		var extension := "res://addons/gdble/gdble.gdextension"
		if FileAccess.file_exists(extension) and not GDExtensionManager.is_extension_loaded(extension):
			GDExtensionManager.load_extension(extension)
	if manager == null and ClassDB.class_exists("BluetoothManager"):
		manager = ClassDB.instantiate("BluetoothManager")
		add_child(manager)
	if manager == null:
		status = "GDBLE 扩展未加载，请检查插件安装后重试"
		changed.emit("adapter", "")
		return
	manager.ble_event.connect(_queue_manager)
	retry_initialize()

func retry_initialize() -> void:
	if manager == null:
		status = "GDBLE 扩展未加载，请检查插件安装并重新启动"
		changed.emit("adapter", "")
		return
	if initializing:
		return
	initializing = true
	status = "正在检查权限并初始化蓝牙…"
	changed.emit("adapter", "")
	if is_instance_valid(_gate):
		remove_child(_gate)
		_gate.queue_free()
	if _use_permissions:
		_gate = PermissionStartup.new()
		add_child(_gate)
		_gate.permission_denied.connect(_permission_denied)
		if manager.is_initialized():
			initializing = false
			ready_for_ble = true
			status = "蓝牙已就绪"
		_gate.start(manager)
	else:
		manager.initialize()

func _permission_denied(missing: PackedStringArray) -> void:
	_invalidate_adapter()
	status = "权限未授予：%s。请授权后点击重试；永久拒绝时请到系统设置开启。" % ", ".join(missing)
	_log("", status, true)
	changed.emit("adapter", "")

func current():
	return sessions.get(selected_address)

func select_device(address: String) -> void:
	selected_address = Session.normalize(address)
	changed.emit("selection", selected_address)

func select_characteristic(service: String, characteristic: String) -> void:
	var session = current()
	if session:
		session.select(service, characteristic)
		changed.emit("selection", session.address)

func start_scan(seconds: float) -> void:
	if not ready_for_ble or scanning or scan_pending:
		return
	scan_pending = true
	scan_id = manager.start_scan(seconds)
	changed.emit("scan", "")

func stop_scan() -> void:
	if not scanning or scan_pending:
		return
	scan_pending = true
	manager.stop_scan()
	changed.emit("scan", "")

func connect_device(address: String) -> void:
	var session = sessions.get(Session.normalize(address))
	if not ready_for_ble or session == null or session.connected or not session.pending.is_empty():
		return
	if session.device == null:
		var native_device = manager.get_or_create_device(session.address)
		if native_device == null:
			_log(session.address, "无法获取设备，请重新扫描", true)
			return
		session.device = _device_factory.call(native_device) if _device_factory.is_valid() else DeviceAccess.new(native_device)
		session.device.ble_event.connect(_queue_device.bind(session.address))
	_submit(session, "connect")

func disconnect_device(address: String) -> void:
	var session = sessions.get(Session.normalize(address))
	if session and session.connected and not session.pending.has("disconnect"):
		_submit(session, "disconnect")

func discover() -> void:
	var session = current()
	if _available(session):
		_submit(session, "discover_services")

func read() -> void:
	var session = current()
	if _available(session) and session.properties().get("read", false):
		_submit(session, "read")

func write() -> Dictionary:
	var session = current()
	if not _available(session):
		return {"ok": false, "error": "请先连接设备并等待当前操作结束"}
	var encoded := Codec.encode(session.draft, session.format)
	if not encoded.ok:
		return encoded
	var props: Dictionary = session.properties()
	if not props.get("write", false) and not props.get("write_without_response", false):
		return {"ok": false, "error": "该特征不支持写入"}
	if not props.get("write", false):
		session.with_response = false
	elif not props.get("write_without_response", false):
		session.with_response = true
	_submit(session, "write", encoded.data)
	return encoded

func toggle_subscription() -> void:
	var session = current()
	if not _available(session):
		return
	var props: Dictionary = session.properties()
	if not props.get("notify", false) and not props.get("indicate", false):
		return
	var subscribed: bool = session.subscriptions.has(session.key(session.service_uuid, session.characteristic_uuid))
	_submit(session, "unsubscribe" if subscribed else "subscribe")

func _available(session) -> bool:
	return ready_for_ble and session != null and session.connected and session.pending.is_empty()

func _submit(session, operation: String, data := PackedByteArray()) -> void:
	var service: String = session.service_uuid
	var characteristic: String = session.characteristic_uuid
	var id: int = session.device.request(operation, service, characteristic, data, session.with_response)
	session.pending[operation] = {"id": id, "service": service, "characteristic": characteristic, "data": data}
	if operation == "connect":
		session.state = "连接中…"
	elif operation == "disconnect":
		session.state = "断开中…"
	changed.emit("session", session.address)

func _queue_manager(event: Dictionary) -> void:
	if str(event.get("operation", "")) in ["initialize", "scan", "resolve_device"]:
		_enqueue(event, "")

func _queue_device(event: Dictionary, address: String) -> void:
	if str(event.get("operation", "")) in DEVICE_OPERATIONS:
		_enqueue(event, address)

func _enqueue(event: Dictionary, address: String) -> void:
	# Some rejected calls emit before returning their operation ID.
	_events.append([event.duplicate(true), address])
	if not _scheduled:
		_scheduled = true
		_drain.call_deferred()

func _drain() -> void:
	_scheduled = false
	var events := _events
	_events = []
	for queued in events:
		if queued[1].is_empty():
			_manager_event(queued[0])
		else:
			_device_event(queued[0], queued[1])

func _manager_event(event: Dictionary) -> void:
	var operation := str(event.get("operation", ""))
	var phase := str(event.get("phase", ""))
	if operation == "initialize":
		if phase == "succeeded":
			ready_for_ble = true
			initializing = false
			status = "蓝牙已就绪 · 点击扫描查找设备"
			_log("", status)
		elif phase == "failed" or phase == "cancelled":
			_invalidate_adapter()
			status = "蓝牙不可用：%s。检查蓝牙开关、权限后重试。" % _error(event)
			_log("", status, true)
		elif phase == "progress" and not ready_for_ble:
			status = "适配器状态已变化，请点击重试初始化"
		changed.emit("adapter", "")
	elif operation == "scan":
		if int(event.get("operation_id", 0)) == 0 and phase == "failed":
			_log("", "扫描状态异常：" + _error(event), true)
			return
		if int(event.get("operation_id", 0)) != scan_id:
			return
		if phase == "started":
			scanning = true
			scan_pending = false
			_log("", "开始扫描")
		elif phase == "progress" and event.get("data") is Dictionary:
			_upsert(event.data)
		elif event.get("terminal", false):
			scanning = false
			scan_pending = false
			_log("", "扫描失败：" + _error(event) if phase == "failed" else "扫描已结束", phase == "failed")
		changed.emit("scan", "")
	elif operation == "resolve_device" and phase == "failed":
		_log(Session.normalize(str(event.get("device_address", ""))), _error(event), true)

func _upsert(info: Dictionary) -> void:
	var address := Session.normalize(str(info.get("address", "")))
	if address.is_empty():
		return
	if not sessions.has(address):
		var session = Session.new()
		session.address = address
		sessions[address] = session
	var session = sessions[address]
	if info.get("name") != null and not str(info.name).is_empty():
		session.name = str(info.name)
	session.rssi = info.get("rssi")
	if selected_address.is_empty():
		selected_address = address
	changed.emit("devices", address)

func _device_event(event: Dictionary, address: String) -> void:
	var session = sessions.get(address)
	if session == null:
		return
	var operation := str(event.get("operation", ""))
	var phase := str(event.get("phase", ""))
	var id := int(event.get("operation_id", 0))
	var request: Dictionary = session.pending.get(operation, {})
	if id != 0 and (request.is_empty() or request.id != id):
		return
	if phase in ["started", "progress"]:
		return
	if event.get("terminal", false):
		session.pending.erase(operation)
	var service := str(event.get("service_uuid", request.get("service", ""))).to_lower()
	var characteristic := str(event.get("characteristic_uuid", request.get("characteristic", ""))).to_lower()
	var key: String = session.key(service, characteristic)
	var error_code := str(event.get("error", {}).get("code", ""))
	if phase == "failed" or phase == "cancelled":
		if error_code == "SERVICES_CHANGED":
			session.invalidate_gatt()
		elif operation == "notification":
			session.subscriptions.clear()
		if not session.device.connection_active():
			session.disconnected()
		else:
			session.connected = true
			session.state = "已连接"
		_log(address, "%s失败：%s%s" % [OPERATION_LABELS.get(operation, operation), _error(event), "；请重新发现服务" if error_code == "SERVICES_CHANGED" else ("；请重新订阅" if operation == "notification" else "")], true)
	elif operation == "connect" and phase == "succeeded":
		session.connected = true
		session.state = "已连接"
		_log(address, "连接成功")
		_submit(session, "discover_services")
	elif operation == "disconnect":
		session.disconnected()
		_log(address, "连接已断开，可重新连接")
	elif operation == "discover_services" and phase == "succeeded":
		session.services = event.get("data", []).duplicate(true)
		if session.properties().is_empty():
			session.service_uuid = ""
			session.characteristic_uuid = ""
		_log(address, "服务发现完成：%d 个服务" % session.services.size())
	elif operation in ["subscribe", "unsubscribe"] and phase == "succeeded":
		if operation == "subscribe":
			session.subscriptions[key] = true
		else:
			session.subscriptions.erase(key)
		_log(address, "%s\n%s / %s" % ["订阅成功" if operation == "subscribe" else "已取消订阅", service, characteristic])
	elif (operation == "read" and phase == "succeeded") or (operation == "notification" and phase == "received" and session.connected and ready_for_ble) or (operation == "write" and phase == "succeeded"):
		var data: PackedByteArray = request.get("data", PackedByteArray()) if operation == "write" else event.get("data", PackedByteArray())
		var time := Time.get_datetime_string_from_system().replace("T", " ")
		var description := "%s\n%s / %s\n%s" % [{"read": "读取", "write": "发送", "notification": "通知"}[operation], service, characteristic, Codec.describe(data)]
		session.latest[key] = time + "  " + description
		_log(address, description)
	changed.emit("session", address)

func _invalidate_adapter() -> void:
	ready_for_ble = false
	initializing = false
	scanning = false
	scan_pending = false
	scan_id = 0
	for session in sessions.values():
		session.disconnected()

func _error(event: Dictionary) -> String:
	var error: Dictionary = event.get("error", {})
	return "%s %s" % [error.get("code", ""), error.get("message", "操作未完成")]

func _log(address: String, message: String, is_error := false) -> void:
	logs.add(address, message, is_error)
	changed.emit("logs", address)
