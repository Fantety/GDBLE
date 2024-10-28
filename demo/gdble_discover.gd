extends GodotBle

signal start_write
# Called when the node enters the scene tree for the first time.
func _ready() -> void:
	print(init_adapter_list())
	set_adapter(0)
	start_scan()
	pass # Replace with function body.


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta: float) -> void:
	pass


func _on_on_device_found(identifier: String, address: String) -> void:
	print(identifier,'\t',address)
	if identifier=='ESP32':
		stop_scan()
		print(get_device_index_from_identifier('ESP32'))
		if connect_to_device(get_device_index_from_identifier('ESP32'))==0:
			print("success connnect")
			pass
		print(show_all_services())
		print(get_current_device_index())
		emit_signal("start_write")
		pass
	pass # Replace with function body.


func _on_start_write() -> void:
	print("start_write")
	print(write_data_to_service(3,"r#hello"))
	pass # Replace with function body.
