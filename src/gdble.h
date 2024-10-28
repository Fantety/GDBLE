/*
 * @FilePath: \src\gdble.h
 * @Author: Fantety
 * @Descripttion: 
 * @Date: 2024-08-28 09:20:26
 * @LastEditors: Fantety
 * @LastEditTime: 2024-10-28 09:51:41
 */
/* godot-cpp integration testing project.
 *
 * This is free and unencumbered software released into the public domain.
 */

#ifndef GDBLE_H
#define GDBLE_H

// We don't need windows.h in this example plugin but many others do, and it can
// lead to annoying situations due to the ton of macros it defines.
// So we include it and make sure CI warns us if we use something that conflicts
// with a Windows define.
#ifdef WIN32
#include <windows.h>
#endif

#include <godot_cpp/classes/control.hpp>
#include <godot_cpp/classes/global_constants.hpp>
#include <godot_cpp/classes/image.hpp>
#include <godot_cpp/classes/input_event_key.hpp>
#include <godot_cpp/classes/tile_map.hpp>
#include <godot_cpp/classes/tile_set.hpp>
#include <godot_cpp/classes/viewport.hpp>
#include <godot_cpp/variant/variant.hpp>

#include <godot_cpp/core/binder_common.hpp>
#include <godot_cpp/core/gdvirtual.gen.inc>

#include "utils.h"

using namespace godot;

class GodotBle : public Node {
	GDCLASS(GodotBle, Node);
private:
	std::vector<SimpleBLE::Adapter> adapters;
	std::vector<SimpleBLE::Peripheral> devices;
	SimpleBLE::Peripheral* current_device = nullptr;
	SimpleBLE::Adapter adapter;
	bool displayed = false;
	std::vector<std::pair<SimpleBLE::BluetoothUUID, SimpleBLE::BluetoothUUID>> uuids;
	std::vector<SimpleBLE::Characteristic> characteristics;
	int current_adapter_index = -1;
	int current_device_index = -1;

protected:
	static void _bind_methods();
	
	void emit_found_signal(SimpleBLE::Peripheral peripheral);
	void emit_update_signal(SimpleBLE::Peripheral peripheral);
	std::string godot_string_to_cpp_string(String godot_string);

public:

	GodotBle();
	~GodotBle();

    bool bluetooth_enabled();
    Dictionary init_adapter_list();
	bool set_adapter(int index);
	void start_scan();
	void stop_scan();
	Dictionary show_all_devices();
	int get_adapters_index_from_identifier(String identifier);
	int get_device_index_from_identifier(String identifier);
	int connect_to_device(int index);
	int disconnect_from_device();
	int get_current_adapter_index();
	int get_current_device_index();
	Dictionary show_all_services();
	String read_data_from_service(int index);
	int write_data_to_service(int index, String data);
};

#endif // EXAMPLE_CLASS_H