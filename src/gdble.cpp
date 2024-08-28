/*
 * @FilePath: \src\gdble.cpp
 * @Author: Fantety
 * @Descripttion: 
 * @Date: 2024-08-28 09:20:10
 * @LastEditors: Fantety
 * @LastEditTime: 2024-08-28 17:07:06
 */
/* godot-cpp integration testing project.
 *
 * This is free and unencumbered software released into the public domain.
 */

#include "gdble.h"

#include <godot_cpp/core/class_db.hpp>

#include <godot_cpp/classes/global_constants.hpp>
#include <godot_cpp/classes/label.hpp>
#include <godot_cpp/classes/multiplayer_api.hpp>
#include <godot_cpp/classes/multiplayer_peer.hpp>
#include <godot_cpp/classes/os.hpp>
#include <godot_cpp/variant/utility_functions.hpp>

using namespace godot;

bool GodotBle::bluetooth_enabled() {
	if (!SimpleBLE::Adapter::bluetooth_enabled()) {
        return true;
    }
    return false;
}

Dictionary GodotBle::init_adapter_list(){
    Dictionary temp;
    adapters.clear();
    adapters = SimpleBLE::Adapter::get_adapters();
    if (adapters.empty()) {
        temp["warning"]="No adapters found";
        return temp;
    }
    for (auto &&i : adapters)
    {
        temp[i.identifier().empty()?"Unknown":i.identifier().c_str()]=i.address().c_str();
    }
    return temp;
}
bool GodotBle::set_adapter(int index){
    if(index<0 || index>= adapters.size())
        return false;
    adapter = adapters[index];
    adapter.set_callback_on_scan_start([]()->String {
        return "Device Scan started\n";
    });
    adapter.set_callback_on_scan_stop([]()->String {
        return "Device Scan stopped\n";
    });
    adapter.set_callback_on_scan_found(std::bind(&GodotBle::emit_found_signal,this,std::placeholders::_1));
    adapter.set_callback_on_scan_updated(std::bind(&GodotBle::emit_update_signal, this, std::placeholders::_1));
    return true;
}

void GodotBle::start_scan(){
    devices.clear();
    adapter.scan_start();
}
void GodotBle::stop_scan(){
    adapter.scan_stop();
}

Dictionary GodotBle::show_all_devices(){
    Dictionary temp;
    if(devices.empty()){
        temp["warning"]="No devices found";
        return temp;
    }
    for (auto &&i : devices)
    {
        temp[i.identifier().empty()?"Unknown":i.identifier().c_str()]=i.address().c_str();
    }
    return temp;
}

void GodotBle::emit_found_signal(SimpleBLE::Peripheral peripheral){
    devices.push_back(peripheral);
    call_deferred("emit_signal", "on_device_found", peripheral.identifier().empty()?"Unknown":peripheral.identifier().c_str(), peripheral.address().c_str());
    //emit_signal("on_device_found", peripheral.identifier().c_str(), peripheral.address().c_str());
}
void GodotBle::emit_update_signal(SimpleBLE::Peripheral peripheral){
    call_deferred("emit_signal", "on_device_update", peripheral.identifier().empty()?"Unknown":peripheral.identifier().c_str(), peripheral.address().c_str());
    //emit_signal("on_device_update", peripheral.identifier().c_str(), peripheral.address().c_str());
}

void GodotBle::_bind_methods() {
	ClassDB::bind_method(D_METHOD("bluetooth_enabled"), &GodotBle::bluetooth_enabled);
    ClassDB::bind_method(D_METHOD("init_adapter_list"), &GodotBle::init_adapter_list);
    ClassDB::bind_method(D_METHOD("start_scan"), &GodotBle::start_scan);
    ClassDB::bind_method(D_METHOD("stop_scan"), &GodotBle::stop_scan);
    ClassDB::bind_method(D_METHOD("set_adapter","index"), &GodotBle::set_adapter,DEFVAL(0));
    ClassDB::bind_method(D_METHOD("show_all_devices"), &GodotBle::show_all_devices);
    ADD_SIGNAL(MethodInfo("on_device_found", PropertyInfo(Variant::STRING, "identifier"), PropertyInfo(Variant::STRING, "address")));
    ADD_SIGNAL(MethodInfo("on_device_update", PropertyInfo(Variant::STRING, "identifier"), PropertyInfo(Variant::STRING, "address")));
}   

GodotBle::GodotBle() {
    
}

GodotBle::~GodotBle() {
}

