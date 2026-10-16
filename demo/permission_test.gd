extends SceneTree

const Startup = preload("res://addons/gdble/permission_startup.gd")

class Manager extends RefCounted:
	var calls := 0
	func is_initialized() -> bool:
		return calls > 0
	func initialize() -> void:
		calls += 1

func _initialize() -> void:
	_run.call_deferred()

func _run() -> void:
	for api in [23, 30, 31, 35]:
		for scenario in ["granted", "first", "partial", "permanent"]:
			var manager := Manager.new()
			var gate = Startup.new()
			root.add_child(gate)
			var granted := []
			var requests := [0]
			var denials := []
			gate.permission_denied.connect(func(missing): denials.append(missing))
			var needed := Startup.required_permissions(api)
			if scenario == "granted":
				granted.assign(needed)
			gate.start(manager, "Android", api, func(): return granted,
				func(): requests[0] += 1; return scenario != "permanent")
			assert(manager.calls == (1 if scenario == "granted" else 0))
			assert(requests[0] == (0 if scenario == "granted" else 1))
			if scenario == "first":
				for permission in needed:
					granted.append(permission)
					gate._on_permission_result(permission, true)
					await process_frame
				assert(manager.calls == 1)
			elif scenario == "partial":
				if needed.size() > 1:
					granted.append(needed[0])
				for permission in needed:
					gate._on_permission_result(permission, granted.has(permission))
				await process_frame
				assert(manager.calls == 0)
				assert(not denials.is_empty())
			elif scenario == "permanent":
				assert(not denials.is_empty())
			for index in range(3):
				gate.notification(Node.NOTIFICATION_APPLICATION_RESUMED)
				await process_frame
			assert(requests[0] <= 1)
			granted.assign(needed) # Permission granted later in Settings.
			gate.notification(Node.NOTIFICATION_APPLICATION_RESUMED)
			await process_frame
			assert(manager.calls == 1)
			gate.queue_free()
			await process_frame
	print("GDBLE permission test passed.")
	quit(0)
