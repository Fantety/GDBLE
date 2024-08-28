/*
 * @FilePath: \src\register_types.cpp
 * @Author: Fantety
 * @Descripttion: 
 * @Date: 2024-08-28 09:12:51
 * @LastEditors: Fantety
 * @LastEditTime: 2024-08-28 14:13:46
 */
/* godot-cpp integration testing project.
 *
 * This is free and unencumbered software released into the public domain.
 */

#include "register_types.h"

#include <gdextension_interface.h>

#include <godot_cpp/core/class_db.hpp>
#include <godot_cpp/core/defs.hpp>
#include <godot_cpp/godot.hpp>

#include "gdble.h"
#include "utils.h"

using namespace godot;

void initialize_gdble_module(ModuleInitializationLevel p_level) {
	if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
		return;
	}

	//GDREGISTER_RUNTIME_CLASS(GodotBle);
	ClassDB::register_class<GodotBle>();
}

void uninitialize_gdble_module(ModuleInitializationLevel p_level) {
	if (p_level != MODULE_INITIALIZATION_LEVEL_SCENE) {
		return;
	}
}

extern "C" {
// Initialization.
GDExtensionBool GDE_EXPORT gdble_library_init(GDExtensionInterfaceGetProcAddress p_get_proc_address, GDExtensionClassLibraryPtr p_library, GDExtensionInitialization *r_initialization) {
	godot::GDExtensionBinding::InitObject init_obj(p_get_proc_address, p_library, r_initialization);

	init_obj.register_initializer(initialize_gdble_module);
	init_obj.register_terminator(uninitialize_gdble_module);
	init_obj.set_minimum_library_initialization_level(MODULE_INITIALIZATION_LEVEL_SCENE);

	return init_obj.init();
}
}