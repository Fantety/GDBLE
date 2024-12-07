/*
 * @FilePath: \src\gdble.cpp
 * @Author: Fantety
 * @Descripttion: 
 * @Date: 2024-08-28 09:20:10
 * @LastEditors: Fantety
 * @LastEditTime: 2024-12-07 11:01:49
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

bool GodotBLE::bluetooth_enabled() {
	if (!SimpleBLE::Adapter::bluetooth_enabled()) {
        return true;
    }
    return false;
}

Dictionary GodotBLE::init_adapter_list(){
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


bool GodotBLE::set_adapter(int index){
    if(index<0 || index>= adapters.size())
        return false;
    adapter = adapters[index];
    current_adapter_index = index;
    adapter.set_callback_on_scan_start(std::bind(&GodotBLE::emit_scan_start,this));
    adapter.set_callback_on_scan_stop(std::bind(&GodotBLE::emit_scan_stop,this));
    adapter.set_callback_on_scan_found(std::bind(&GodotBLE::emit_found_signal,this,std::placeholders::_1));
    adapter.set_callback_on_scan_updated(std::bind(&GodotBLE::emit_update_signal, this, std::placeholders::_1));
    return true;
}


int GodotBLE::get_adapters_index_from_identifier(String identifier){
    for (int i = 0; i < adapters.size(); i++)
    {
        if(adapters[i].identifier().empty() && adapters[i].address().empty())
            continue;
        if(String{adapters[i].identifier().c_str()}==identifier)
            return i;
    }
    return -1;
}

int GodotBLE::get_device_index_from_identifier(String identifier){
    for (int i = 0; i < devices.size(); i++)
    {
        if(devices[i].identifier().empty() && devices[i].address().empty())
            continue;
        if(String{devices[i].identifier().c_str()}==identifier)
            return i;
    }
    return -1;
}

int GodotBLE::get_current_adapter_index(){
    return current_adapter_index;
}

int GodotBLE::get_current_device_index(){
    return current_device_index;
}

void GodotBLE::start_scan(){
    devices.clear();
    adapter.scan_start();
}
void GodotBLE::stop_scan(){
    adapter.scan_stop();
}

Dictionary GodotBLE::show_all_devices(){
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

int GodotBLE::connect_to_device(int index){
    if(index<0 || index>= devices.size())
        return -1;
    if(devices[index].is_connected())
        return -2;
    devices[index].connect();
    current_device = &devices[index];
    current_device_index = index;
    uuids.clear();
    for (auto service : current_device->services()) {
        for (auto characteristic : service.characteristics()) {
            uuids.push_back(std::make_pair(service.uuid(), characteristic.uuid()));
            characteristics.push_back(characteristic);
        }
    }
    return 0;
}

int GodotBLE::disconnect_from_device(){
    if(current_device_index<0 || current_device_index>= devices.size())
        return -1;
    if(!current_device->is_connected())
        return -2;
    current_device->disconnect();
    current_device = nullptr;
    current_device_index = -1;
    return 0;
}

Dictionary GodotBLE::show_all_services(){
    Dictionary temp;
    if(!current_device->is_connected()){
        temp["warning"]="Device is not connected";
        return temp;
    }
    if(uuids.empty()){
        temp["warning"]="No devices found";
        return temp;
    }
    for (auto &&i : uuids)
    {
        temp[i.first.c_str()]=i.second.c_str();
    }
    return temp;
}

String GodotBLE::read_data_from_service(int index){
    if(!current_device->is_connected()){
        return "Device is not connected";
    }
    if(index<0 || index>= uuids.size())
        return "Invalid index";
    SimpleBLE::ByteArray rx_data = current_device->read(uuids[index].first, uuids[index].second);
    return String{rx_data.c_str()};
}

std::string GodotBLE::godot_string_to_cpp_string(String str){
    // int length = str.utf8().length();
    // // 使用std::string的构造函数直接从Godot字符串的UTF-8表示创建std::string
    // return std::string(str.utf8().get_data(), length);
    return "";
}

int GodotBLE::write_data_to_service(int index, String data){
    if(!current_device->is_connected()){
        return -1;
    }
    if(index<0 || index>= uuids.size())
        return -2;
    std::string cpp_data = data.utf8().get_data();
    SimpleBLE::ByteArray bytes = cpp_data;
    if (current_device != nullptr)
    {
        if(characteristics[index].can_write_command()){
            current_device->write_command(uuids[index].first, uuids[index].second, bytes);
            return 0;
        }
        else if(characteristics[index].can_write_request()){
            current_device->write_request(uuids[index].first, uuids[index].second, bytes);
            return 0;
        }
        else{
            return -4;
        }
        /* code */
    }
    return -3;
}
int GodotBLE::subscribe_notify(int index){
    if(!current_device->is_connected()){
        return -1;
    }
    if(index<0 || index>= uuids.size())
        return -2;
    current_device->notify(uuids[index].first, uuids[index].second,
        std::bind(&GodotBLE::emit_notified, this, std::placeholders::_1));
}

void GodotBLE::emit_found_signal(SimpleBLE::Peripheral peripheral){
    devices.push_back(peripheral);
    call_deferred("emit_signal", "device_found", peripheral.identifier().empty()?"Unknown":peripheral.identifier().c_str(), peripheral.address().c_str());
    //emit_signal("on_device_found", peripheral.identifier().c_str(), peripheral.address().c_str());
}
void GodotBLE::emit_update_signal(SimpleBLE::Peripheral peripheral){
    call_deferred("emit_signal", "device_update", peripheral.identifier().empty()?"Unknown":peripheral.identifier().c_str(), peripheral.address().c_str());
    //emit_signal("on_device_update", peripheral.identifier().c_str(), peripheral.address().c_str());
}

void GodotBLE::emit_scan_stop(){
    call_deferred("emit_signal", "scan_stop");
    //emit_signal("scan_stop");
}
void GodotBLE::emit_scan_start(){
    call_deferred("emit_signal", "scan_start");
    //emit_signal("scan_start");
}
void GodotBLE::emit_notified(SimpleBLE::ByteArray bytes){
    call_deferred("emit_signal", "notified",String{bytes.c_str()});
}


void GodotBLE::_bind_methods() {
	ClassDB::bind_method(D_METHOD("bluetooth_enabled"), &GodotBLE::bluetooth_enabled);
    ClassDB::bind_method(D_METHOD("init_adapter_list"), &GodotBLE::init_adapter_list);
    ClassDB::bind_method(D_METHOD("start_scan"), &GodotBLE::start_scan);
    ClassDB::bind_method(D_METHOD("stop_scan"), &GodotBLE::stop_scan);
    ClassDB::bind_method(D_METHOD("get_adapters_index_from_identifier","identifier"), &GodotBLE::get_adapters_index_from_identifier);
    ClassDB::bind_method(D_METHOD("get_device_index_from_identifier","identifier"), &GodotBLE::get_device_index_from_identifier);
    ClassDB::bind_method(D_METHOD("set_adapter","index"), &GodotBLE::set_adapter,DEFVAL(0));
    ClassDB::bind_method(D_METHOD("show_all_devices"), &GodotBLE::show_all_devices);
    ClassDB::bind_method(D_METHOD("connect_to_device","index"), &GodotBLE::connect_to_device,DEFVAL(0));
    ClassDB::bind_method(D_METHOD("get_current_adapter_index"), &GodotBLE::get_current_adapter_index);
    ClassDB::bind_method(D_METHOD("get_current_device_index"), &GodotBLE::get_current_device_index);
    ClassDB::bind_method(D_METHOD("show_all_services"), &GodotBLE::show_all_services);
    ClassDB::bind_method(D_METHOD("read_data_from_service","index"), &GodotBLE::read_data_from_service,DEFVAL(0));
    ClassDB::bind_method(D_METHOD("write_data_to_service","index","data"), &GodotBLE::write_data_to_service);
    ClassDB::bind_method(D_METHOD("subscribe_notify","index"), &GodotBLE::subscribe_notify);
    ADD_SIGNAL(MethodInfo("device_found", PropertyInfo(Variant::STRING, "identifier"), PropertyInfo(Variant::STRING, "address")));
    ADD_SIGNAL(MethodInfo("device_update", PropertyInfo(Variant::STRING, "identifier"), PropertyInfo(Variant::STRING, "address")));
    ADD_SIGNAL(MethodInfo("scan_start"));
    ADD_SIGNAL(MethodInfo("scan_stop"));
    ADD_SIGNAL(MethodInfo("notified", PropertyInfo(Variant::STRING, "data")));
}   

GodotBLE::GodotBLE() {
    
}

GodotBLE::~GodotBLE() {
}

