# GodotBle

A Low Power Bluetooth Plugin for Godot based on SimpleBLE

### Overview

The plugin is based on [SimpleBLE](https://github.com/OpenBluetoothToolbox/SimpleBLE), uses GDExtension, and only supports low-power Bluetooth

### Supported platforms

| Platform | Versions                |
| -------- | ----------------------- |
| Windows  | Windows 10+             |
| Linux    | Temporarily unsupported |
| MacOS    | Temporarily unsupported |

### New Node

| Node     | Notes                                     |
| -------- | ----------------------------------------- |
| GodotBle | All functions are integrated in this node |

### How to use in GDScriptH

Functions provided by the GodotBle nodeF

| Function                                              | Notes                                                                                                                                                                                     |
| ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| init_adapter_list()                                   | Initialize the list of adapters, all adapters are stored in the node's private member variables.?The return value of this function is a dictionary containing all the adapter information |
| set_adapter(int index)                                | Select Adapter, pass in an adapter index, default value is 0.                                                                                                                             |
| start_scan()                                          | Start scanning the environment for Bluetooth devices.                                                                                                                                     |
| stop_scan()                                           | Stop scanning for surrounding Bluetooth devices.                                                                                                                                          |
| bluetooth_enabled()                                   | Returns a boolean value that determines whether the Bluetooth adapter is enabled or not.                                                                                                  |
| get_adapters_index_from_identifier(String identifier) | Get its index in the adapter list by adapter name.                                                                                                                                        |
| get_device_index_from_identifier(String identifier)   | Get its index in the device list by device name.                                                                                                                                          |
| connect_to_device(int index)                          | Connect to the specified Bluetooth device.                                                                                                                                                |
| show_all_services()                                   | Displays all services for connected Bluetooth devices.                                                                                                                                    |
| show_all_devices()                                    | Displays all devices in the current device list.                                                                                                                                          |
| get_current_adapter_index()                           | Get the index of the currently selected adapter.                                                                                                                                          |
| get_current_device_index()                            | Get the index of the currently connected device.                                                                                                                                          |

Signals provided by the GodotBle nodeF

| Signal           | Notes                                                                                                                                  |
| ---------------- | -------------------------------------------------------------------------------------------------------------------------------------- |
| on_device_found  | This signal is triggered when a new device is discovered.?Returns two strings, one for the device name and one for the device address. |
| on_device_update | Triggers this signal when device information is updated.?Returns two strings, one for the device name and one for the device address.  |

```gdscript
extends GodotBle
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
```