/*
 * @FilePath: \src\gdble.h
 * @Author: Fantety
 * @Descripttion: 
 * @Date: 2024-08-28 09:20:26
 * @LastEditors: Fantety
 * @LastEditTime: 2024-08-28 17:20:10
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

#include <simpleble/SimpleBLE.h>

using namespace godot;

class GodotBle : public Node {
	GDCLASS(GodotBle, Node);
private:
	std::vector<SimpleBLE::Adapter> adapters;
	std::vector<SimpleBLE::Peripheral> devices;
	SimpleBLE::Adapter adapter;
	bool displayed = false;

protected:
	static void _bind_methods();
	
	void emit_found_signal(SimpleBLE::Peripheral peripheral);
	void emit_update_signal(SimpleBLE::Peripheral peripheral);

public:
	GodotBle();
	~GodotBle();

    bool bluetooth_enabled();
    Dictionary init_adapter_list();
	bool set_adapter(int index);
	void start_scan();
	void stop_scan();
	Dictionary show_all_devices();
};

#endif // EXAMPLE_CLASS_H