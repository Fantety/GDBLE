extends GDBLEDiscover


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
	print(identifier,address,'\n')
	pass # Replace with function body.
