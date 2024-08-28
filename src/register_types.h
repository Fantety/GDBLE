/*
 * @FilePath: \src\register_types.h
 * @Author: Fantety
 * @Descripttion: 
 * @Date: 2024-08-28 09:12:51
 * @LastEditors: Fantety
 * @LastEditTime: 2024-08-28 12:04:31
 */
/* godot-cpp integration testing project.
 *
 * This is free and unencumbered software released into the public domain.
 */

#ifndef EXAMPLE_REGISTER_TYPES_H
#define EXAMPLE_REGISTER_TYPES_H

#include <godot_cpp/core/class_db.hpp>

using namespace godot;

void initialize_gdble_module(ModuleInitializationLevel p_level);
void uninitialize_gdble_module(ModuleInitializationLevel p_level);

#endif // EXAMPLE_REGISTER_TYPES_H