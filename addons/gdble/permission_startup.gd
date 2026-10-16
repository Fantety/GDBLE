extends Node
## Shared permission gate. Add to the scene, then call start(manager).
## Foreground checks never open another dialog; explicit retry is app-controlled.

signal permission_denied(missing: PackedStringArray)

var _manager
var _required := PackedStringArray()
var _read_granted: Callable
var _request: Callable
var _requested := false
var _submitted := false
var _pending := {}
var _active := false

static func required_permissions(api_level: int) -> PackedStringArray:
	if api_level >= 31:
		return PackedStringArray(["android.permission.BLUETOOTH_SCAN", "android.permission.BLUETOOTH_CONNECT"])
	if api_level >= 23:
		return PackedStringArray(["android.permission.ACCESS_FINE_LOCATION"])
	return PackedStringArray()

func start(manager, platform_name := OS.get_name(), api_level := -1,
		granted_reader := Callable(OS, "get_granted_permissions"),
		requester := Callable(OS, "request_permissions")) -> void:
	_manager = manager
	_read_granted = granted_reader
	_request = requester
	if platform_name == "Android":
		if api_level < 0:
			if not Engine.has_singleton("GDBLE") or not Engine.get_singleton("GDBLE").has_method("getAndroidApiLevel"):
				permission_denied.emit(PackedStringArray(["GDBLE Android plugin/API level unavailable"]))
				return
			api_level = Engine.get_singleton("GDBLE").getAndroidApiLevel()
		_required = required_permissions(api_level)
		if not get_tree().on_request_permissions_result.is_connected(_on_permission_result):
			get_tree().on_request_permissions_result.connect(_on_permission_result)
	_active = true
	check_permissions(true)

func check_permissions(allow_prompt := false) -> void:
	if not _active or not is_instance_valid(_manager):
		return
	var granted = _read_granted.call() if not _required.is_empty() else []
	var missing := PackedStringArray()
	for permission in _required:
		if not granted.has(permission):
			missing.append(permission)
	if missing.is_empty():
		_pending.clear()
		if not _submitted and not _manager.is_initialized():
			_submitted = true
			_manager.initialize()
		return
	_submitted = false
	if allow_prompt and not _requested:
		_requested = true
		for permission in missing:
			_pending[permission] = true
		if not _request.call():
			_pending.clear()
			permission_denied.emit(missing)
	elif _pending.is_empty():
		permission_denied.emit(missing)

func _on_permission_result(permission: String, _granted: bool) -> void:
	_pending.erase(permission)
	# Re-read the complete permission set after Godot updates its OS snapshot.
	check_permissions.call_deferred(false)

func _notification(what: int) -> void:
	if what == NOTIFICATION_APPLICATION_RESUMED and _active:
		_pending.clear()
		check_permissions.call_deferred(false)
