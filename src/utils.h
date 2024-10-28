/*
 * @FilePath: \src\utils.h
 * @Author: Fantety
 * @Descripttion: 
 * @Date: 2024-08-28 09:23:11
 * @LastEditors: Fantety
 * @LastEditTime: 2024-08-28 09:23:30
 */
/* godot-cpp integration testing project.
 *
 * This is free and unencumbered software released into the public domain.
 */

#ifndef UTILS_H
#define UTILS_H

#include <godot_cpp/templates/cowdata.hpp>
#include <godot_cpp/templates/hash_map.hpp>
#include <godot_cpp/templates/hash_set.hpp>
#include <godot_cpp/templates/hashfuncs.hpp>
#include <godot_cpp/templates/list.hpp>
#include <godot_cpp/templates/local_vector.hpp>
#include <godot_cpp/templates/pair.hpp>
#include <godot_cpp/templates/rb_map.hpp>
#include <godot_cpp/templates/rb_set.hpp>
#include <godot_cpp/templates/rid_owner.hpp>
#include <godot_cpp/templates/safe_refcount.hpp>
#include <godot_cpp/templates/search_array.hpp>
#include <godot_cpp/templates/self_list.hpp>
#include <godot_cpp/templates/sort_array.hpp>
#include <godot_cpp/templates/spin_lock.hpp>
#include <godot_cpp/templates/thread_work_pool.hpp>
#include <godot_cpp/templates/vector.hpp>
#include <godot_cpp/templates/vmap.hpp>
#include <godot_cpp/templates/vset.hpp>
#include <optional>
#include "simpleble/SimpleBLE.h"

namespace Utils {
/**
 * @brief Function to retrieve the adapter easily
 *
 * @return the adapter or an empty optional
 */
std::optional<SimpleBLE::Adapter> getAdapter();

/**
 * @brief Function to get a user input as size_t, it's used in all examples to select an index
 *
 * @param line the text asking for something to the user
 * @param max the maximum value that can be written
 *
 * @return the value or empty optional
 */
std::optional<std::size_t> getUserInputInt(const std::string& line, std::size_t max);

}  // namespace Utils



#endif // UTILS_H