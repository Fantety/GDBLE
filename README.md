<div align="center">
  <h1>GodotBLE</h1>
<p>
<img src="https://img.picui.cn/free/2024/10/28/671f1110963c8.png"/>
</p>
<p>
    <img lt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/Fantety/GodotBLE/builds.yml">
    <img alt="GitHub code size in bytes" src="https://img.shields.io/github/languages/code-size/Fantety/GodotBLE">
    <img alt="GitHub language count" src="https://img.shields.io/github/languages/count/Fantety/GodotBLE">
<!--     <img src="https://img.shields.io/github/package-json/v/Ritusan/color-library" alt="version" /> -->
  </p>
  <p><i></i></p>
</div>
<br />



一个为Godot4.0开发的低功率蓝牙插件，可以帮助你的游戏联合更多有趣的设备

### 概述

这个插件依赖于 [SimpleBLE](https://github.com/OpenBluetoothToolbox/SimpleBLE), 使用GDExtension, 但是仅仅支持低功率蓝牙，并不支持经典蓝牙。

### 平台支持

| 平台      | 版本          |
| ------- | ----------- |
| Windows | Windows 10+ |
| Linux   | 暂时不支持       |
| MacOS   | 暂时不支持       |

### 提供的节点

| 节点       | 描述              |
| -------- | --------------- |
| GodotBle | 所有功能都包含在这一个节点当中 |

### 怎样在 GDScript 中使用GodotBLE?

在 GodotBle 节点中提供以下一些函数及方法: 

| 方法                                                    | 描述                     |
| ----------------------------------------------------- | ---------------------- |
| init_adapter_list()                                   | 用于初始化蓝牙设备适配器           |
| set_adapter(int index)                                | 选择适配器                  |
| start_scan()                                          | 开始扫描环境当中的蓝牙设备          |
| stop_scan()                                           | 停止扫描环境当中的蓝牙设备          |
| bluetooth_enabled()                                   | 返回一个布尔值，用于判断蓝牙适配器是否启用。 |
| get_adapters_index_from_identifier(String identifier) | 根据适配器名称获取其在适配器列表中的索引。  |
| get_device_index_from_identifier(String identifier)   | 根据设备名称获取其在设备列表中的索引。    |
| connect_to_device(int index)                          | 连接到指定的蓝牙设备。            |
| show_all_services()                                   | 显示已连接蓝牙设备的所有服务。        |
| show_all_devices()                                    | 显示当前设备列表中的所有设备。        |
| get_current_adapter_index()                           | 获取当前所选适配器的索引。          |
| get_current_device_index()                            | 获取当前连接设备的索引。           |

由 GodotBle 节点提供的信号：

| 信号               | 描述                                     |
| ---------------- | -------------------------------------- |
| on_device_found  | 发现新设备时触发该信号。返回两个字符串，一个是设备名称，另一个是设备地址。  |
| on_device_update | 设备信息更新时触发该信号。返回两个字符串，一个是设备名称，另一个是设备地址。 |

示例代码：

```gdscript
extends GodotBle

signal start_write

func _ready() -> void:
    print(init_adapter_list())
    set_adapter(0)
    start_scan()
    pass

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
```