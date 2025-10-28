<div align="center">
  <h1>GDBLE - Godot Bluetooth Low Energy Plugin</h1>
<p>
<img src="GodotBLEx229.png"/>
</p>
<p>
    <img alt="GitHub Actions Workflow Status" src="https://img.shields.io/github/actions/workflow/status/Fantety/GDBLE/release.yml">
    <img alt="GitHub code size in bytes" src="https://img.shields.io/github/languages/code-size/Fantety/GDBLE">
    <img alt="GitHub language count" src="https://img.shields.io/github/languages/count/Fantety/GDBLE">
  </p>
  <p><i>A modern Bluetooth Low Energy (BLE) plugin for Godot 4</i></p>
  <p>
    <a href="README.md">🇨🇳 中文</a> | 
    <a href="README_EN.md">🇺🇸 English</a>
  </p>
</div>

---

## 📖 Table of Contents

- [📖 Table of Contents](#-table-of-contents)
- [Overview](#overview)
  - [Why Choose GDBLE?](#why-choose-gdble)
- [Features](#features)
  - [Core Functionality](#core-functionality)
  - [Technical Features](#technical-features)
- [Platform Support](#platform-support)
- [Installation](#installation)
  - [Method 1: Install from Asset Library (Recommended)](#method-1-install-from-asset-library-recommended)
  - [Method 2: Manual Installation](#method-2-manual-installation)
- [Quick Start](#quick-start)
  - [Basic Usage](#basic-usage)
  - [Connect and Read/Write Data](#connect-and-readwrite-data)
- [API Reference](#api-reference)
  - [BluetoothManager](#bluetoothmanager)
    - [Methods](#methods)
    - [Signals](#signals)
  - [BleDevice](#bledevice)
    - [Methods](#methods-1)
    - [Signals](#signals-1)
- [Complete Examples](#complete-examples)
  - [Example 1: Scan and Connect to Device](#example-1-scan-and-connect-to-device)
  - [Example 2: Read Sensor Data](#example-2-read-sensor-data)
  - [Example 3: Control Smart Light Bulb](#example-3-control-smart-light-bulb)
- [Debug Mode](#debug-mode)
  - [Enable Debug Mode](#enable-debug-mode)
  - [Debug Output Example](#debug-output-example)
  - [Disable Debug Mode](#disable-debug-mode)
  - [Conditional Debugging](#conditional-debugging)
- [FAQ](#faq)
  - [Q: Why can't I find any devices?](#q-why-cant-i-find-any-devices)
  - [Q: What if device connection fails?](#q-what-if-device-connection-fails)
  - [Q: How to handle device disconnection?](#q-how-to-handle-device-disconnection)
  - [Q: Write operation has no response?](#q-write-operation-has-no-response)
  - [Q: How to find correct service and characteristic UUIDs?](#q-how-to-find-correct-service-and-characteristic-uuids)
  - [Q: Does it support connecting to multiple devices simultaneously?](#q-does-it-support-connecting-to-multiple-devices-simultaneously)
  - [Q: How's the performance? Will it affect game frame rate?](#q-hows-the-performance-will-it-affect-game-frame-rate)
- [Data Format Description](#data-format-description)
  - [DeviceInfo Dictionary](#deviceinfo-dictionary)
  - [Service Dictionary](#service-dictionary)
  - [Characteristic Dictionary](#characteristic-dictionary)
- [Best Practices](#best-practices)
  - [1. Error Handling](#1-error-handling)
  - [2. Resource Cleanup](#2-resource-cleanup)
  - [3. Timeout Handling](#3-timeout-handling)
  - [4. State Management](#4-state-management)
  - [5. Data Validation](#5-data-validation)
- [Performance Optimization Tips](#performance-optimization-tips)
  - [1. Scan Optimization](#1-scan-optimization)
  - [2. Write Optimization](#2-write-optimization)
  - [3. Notification Optimization](#3-notification-optimization)
- [Troubleshooting](#troubleshooting)
  - [Enable Detailed Logging](#enable-detailed-logging)
  - [Check Adapter Status](#check-adapter-status)
  - [Verify Device Connection](#verify-device-connection)
  - [View Discovered Services](#view-discovered-services)
- [Contributing](#contributing)
- [License](#license)
- [Acknowledgments](#acknowledgments)
- [Contact](#contact)

---

## Overview

GDBLE is a Bluetooth Low Energy (BLE) plugin designed for Godot 4, built with Rust and GDExtension. Based on the [btleplug](https://github.com/deviceplug/btleplug) library, it provides complete BLE functionality including device scanning, connection, service discovery, characteristic read/write, and notification subscription.

**Plugin Page**: [GDBLE - Godot Asset Library](https://godotengine.org/asset-library/asset/3439)

### Why Choose GDBLE?

- ✅ **Modern Architecture**: Built with Rust and async runtime for excellent performance
- ✅ **Real-time Response**: Non-blocking operations that don't affect game frame rate
- ✅ **Complete Features**: Supports all BLE operations including scan, connect, read, write, and notify
- ✅ **Easy to Use**: Clean GDScript API with signal-driven event system
- ✅ **Debuggable**: Built-in debug mode for development and troubleshooting

---

## Features

### Core Functionality

- 🔍 **Device Scanning**: Scan nearby BLE devices, get device name, address, and signal strength
- 🔗 **Device Connection**: Connect to specified BLE devices
- 🔎 **Service Discovery**: Automatically discover all GATT services and characteristics
- 📖 **Characteristic Reading**: Read characteristic values
- ✍️ **Characteristic Writing**: Write data to characteristics (with/without response)
- 🔔 **Notification Subscription**: Subscribe to characteristic notifications for real-time data
- 🎯 **Multi-device Management**: Manage multiple BLE device connections simultaneously

### Technical Features

- ⚡ **Async Non-blocking**: All operations execute asynchronously without blocking main thread
- 🔒 **Thread Safe**: Uses Tokio runtime to ensure thread safety
- 📊 **Signal Driven**: Event notification through Godot signal system
- 🐛 **Debuggable**: Optional debug mode with detailed logging
- 🎮 **Game Friendly**: Optimized for game development, no frame rate impact

---

## Platform Support

| Platform | Status | Version Required |
| -------- | ------ | ---------------- |
| Windows  | ✅     | Windows 10+      |
| macOS    | ✅     | macOS 10.15+     |
| Linux    | 🚧     | In Development   |

---

## Installation

### Method 1: Install from Asset Library (Recommended)

1. Open **AssetLib** in Godot Editor
2. Search for "GDBLE" or "Bluetooth"
3. Click download and install

### Method 2: Manual Installation

1. Download the latest version from [Releases](https://github.com/Fantety/GodotBLE/releases)
2. Extract to your Godot project's `addons` folder
3. Ensure the file structure is as follows:

```
your_project/
├── addons/
│   └── gdble/
│       ├── gdble.gdextension
│       └── gdble.dll (Windows) or libgdble.dylib (macOS)
```

4. Restart Godot Editor

---

## Quick Start

### Basic Usage

```gdscript
extends Node

var bluetooth_manager: BluetoothManager

func _ready():
    # 1. Create BluetoothManager instance
    bluetooth_manager = BluetoothManager.new()
    add_child(bluetooth_manager)
    
    # 2. Connect signals
    bluetooth_manager.adapter_initialized.connect(_on_adapter_initialized)
    bluetooth_manager.device_discovered.connect(_on_device_discovered)
    bluetooth_manager.scan_stopped.connect(_on_scan_stopped)
    
    # 3. Initialize Bluetooth adapter
    bluetooth_manager.initialize()

func _on_adapter_initialized(success: bool, error: String):
    if success:
        print("Bluetooth initialized successfully")
        # Start scanning for 10 seconds
        bluetooth_manager.start_scan(10.0)
    else:
        print("Bluetooth initialization failed: ", error)

func _on_device_discovered(device_info: Dictionary):
    print("Device found: ", device_info.get("name", "Unknown"))
    print("  Address: ", device_info.get("address"))
    print("  Signal strength: ", device_info.get("rssi"), " dBm")

func _on_scan_stopped():
    print("Scan complete")
    var devices = bluetooth_manager.get_discovered_devices()
    print("Total devices found: ", devices.size())
```

### Connect and Read/Write Data

```gdscript
var connected_device: BleDevice = null

func connect_to_device(address: String):
    # Connect to device
    connected_device = bluetooth_manager.connect_device(address)
    if connected_device:
        # Connect device signals
        connected_device.connected.connect(_on_device_connected)
        connected_device.services_discovered.connect(_on_services_discovered)
        connected_device.characteristic_written.connect(_on_characteristic_written)
        
        # Start connection
        connected_device.connect_async()


func _on_device_connected():
    print("Device connected")
    # Discover services
    connected_device.discover_services()

func _on_services_discovered(services: Array):
    print("Discovered ", services.size(), " services")
    
    # Iterate through services and characteristics
    for service in services:
        var service_uuid = service.get("uuid")
        var characteristics = service.get("characteristics", [])
        
        for characteristic in characteristics:
            var char_uuid = characteristic.get("uuid")
            var properties = characteristic.get("properties", {})
            
            # If characteristic supports write, write data
            if properties.get("write", false):
                var data = "Hello BLE".to_utf8_buffer()
                connected_device.write_characteristic(service_uuid, char_uuid, data, false)

func _on_characteristic_written(char_uuid: String):
    print("Data written successfully: ", char_uuid)
```

---

## API Reference

### BluetoothManager

Bluetooth manager responsible for adapter initialization, device scanning, and connection management.

#### Methods

| Method | Parameters | Return | Description |
|--------|------------|--------|-------------|
| `initialize()` | None | void | Initialize Bluetooth adapter |
| `is_initialized()` | None | bool | Check if adapter is initialized |
| `start_scan(timeout_seconds)` | float | void | Start scanning for devices |
| `stop_scan()` | None | void | Stop scanning |
| `get_discovered_devices()` | None | Array[Dictionary] | Get list of discovered devices |
| `connect_device(address)` | String | BleDevice | Connect to specified device |
| `disconnect_device(address)` | String | void | Disconnect specified device |
| `get_device(address)` | String | BleDevice | Get connected device instance |
| `get_connected_devices()` | None | Array[BleDevice] | Get all connected devices |
| `set_debug_mode(enabled)` | bool | void | Enable/disable debug mode |
| `is_debug_mode()` | None | bool | Check debug mode status |

#### Signals

| Signal | Parameters | Description |
|--------|------------|-------------|
| `adapter_initialized` | success: bool, error: String | Adapter initialization complete |
| `device_discovered` | device_info: Dictionary | New device discovered |
| `device_updated` | device_info: Dictionary | Device information updated |
| `scan_started` | None | Scan started |
| `scan_stopped` | None | Scan stopped |
| `device_connecting` | address: String | Device connection initiated |
| `device_connected` | address: String | Device connected successfully |
| `device_disconnected` | address: String | Device disconnected |
| `error_occurred` | error_message: String | Error occurred |


### BleDevice

Represents a single BLE device, provides connection, service discovery, and data read/write functionality.

#### Methods

| Method | Parameters | Return | Description |
|--------|------------|--------|-------------|
| `connect_async()` | None | void | Asynchronously connect to device |
| `disconnect()` | None | void | Disconnect from device |
| `is_connected()` | None | bool | Check if connected |
| `get_address()` | None | String | Get device address |
| `get_name()` | None | String | Get device name |
| `discover_services()` | None | void | Discover device services |
| `get_services()` | None | Array[Dictionary] | Get discovered services list |
| `read_characteristic(service_uuid, char_uuid)` | String, String | void | Read characteristic value |
| `write_characteristic(service_uuid, char_uuid, data, with_response)` | String, String, PackedByteArray, bool | void | Write characteristic value |
| `subscribe_characteristic(service_uuid, char_uuid)` | String, String | void | Subscribe to characteristic notifications |
| `unsubscribe_characteristic(service_uuid, char_uuid)` | String, String | void | Unsubscribe from notifications |

#### Signals

| Signal | Parameters | Description |
|--------|------------|-------------|
| `connected` | None | Device connected successfully |
| `disconnected` | None | Device disconnected |
| `connection_failed` | error: String | Connection failed |
| `services_discovered` | services: Array | Service discovery complete |
| `characteristic_read` | char_uuid: String, data: PackedByteArray | Characteristic read complete |
| `characteristic_written` | char_uuid: String | Characteristic write complete |
| `characteristic_notified` | char_uuid: String, data: PackedByteArray | Characteristic notification received |
| `operation_failed` | operation: String, error: String | Operation failed |

---

## Complete Examples

### Example 1: Scan and Connect to Device

```gdscript
extends Node

var bluetooth_manager: BluetoothManager
var target_device_name = "MyDevice"

func _ready():
    bluetooth_manager = BluetoothManager.new()
    add_child(bluetooth_manager)
    
    bluetooth_manager.adapter_initialized.connect(_on_initialized)
    bluetooth_manager.device_discovered.connect(_on_device_found)
    bluetooth_manager.scan_stopped.connect(_on_scan_done)
    
    bluetooth_manager.initialize()

func _on_initialized(success: bool, error: String):
    if success:
        bluetooth_manager.start_scan(10.0)

func _on_device_found(info: Dictionary):
    var name = info.get("name", "")
    if name == target_device_name:
        print("Target device found!")
        bluetooth_manager.stop_scan()
        connect_to_target(info.get("address"))


func _on_scan_done():
    print("Scan complete")

func connect_to_target(address: String):
    var device = bluetooth_manager.connect_device(address)
    if device:
        device.connected.connect(_on_connected)
        device.connect_async()

func _on_connected():
    print("Device connected!")
```

### Example 2: Read Sensor Data

```gdscript
extends Node

var bluetooth_manager: BluetoothManager
var sensor_device: BleDevice

# Standard Heart Rate Service UUID
const HEART_RATE_SERVICE = "0000180d-0000-1000-8000-00805f9b34fb"
const HEART_RATE_MEASUREMENT = "00002a37-0000-1000-8000-00805f9b34fb"

func _ready():
    bluetooth_manager = BluetoothManager.new()
    add_child(bluetooth_manager)
    
    bluetooth_manager.adapter_initialized.connect(_on_initialized)
    bluetooth_manager.device_discovered.connect(_on_device_found)
    
    bluetooth_manager.initialize()

func _on_initialized(success: bool, error: String):
    if success:
        bluetooth_manager.start_scan(10.0)

func _on_device_found(info: Dictionary):
    # Look for heart rate monitor
    var name = info.get("name", "")
    if "Heart" in name or "HR" in name:
        bluetooth_manager.stop_scan()
        connect_to_sensor(info.get("address"))

func connect_to_sensor(address: String):
    sensor_device = bluetooth_manager.connect_device(address)
    if sensor_device:
        sensor_device.connected.connect(_on_sensor_connected)
        sensor_device.services_discovered.connect(_on_services_found)
        sensor_device.characteristic_notified.connect(_on_heart_rate_update)
        sensor_device.connect_async()

func _on_sensor_connected():
    print("Sensor connected")
    sensor_device.discover_services()

func _on_services_found(services: Array):
    # Subscribe to heart rate notifications
    sensor_device.subscribe_characteristic(HEART_RATE_SERVICE, HEART_RATE_MEASUREMENT)

func _on_heart_rate_update(char_uuid: String, data: PackedByteArray):
    if char_uuid.to_lower() == HEART_RATE_MEASUREMENT:
        # Parse heart rate data (simplified)
        if data.size() > 1:
            var heart_rate = data[1]
            print("Current heart rate: ", heart_rate, " BPM")
```


### Example 3: Control Smart Light Bulb

```gdscript
extends Node

var bluetooth_manager: BluetoothManager
var light_device: BleDevice

# Custom service UUID (example)
const LIGHT_SERVICE = "0000fff0-0000-1000-8000-00805f9b34fb"
const LIGHT_CONTROL = "0000fff2-0000-1000-8000-00805f9b34fb"

func _ready():
    bluetooth_manager = BluetoothManager.new()
    add_child(bluetooth_manager)
    
    bluetooth_manager.adapter_initialized.connect(_on_initialized)
    bluetooth_manager.initialize()

func _on_initialized(success: bool, error: String):
    if success:
        bluetooth_manager.start_scan(10.0)

func connect_to_light(address: String):
    light_device = bluetooth_manager.connect_device(address)
    if light_device:
        light_device.connected.connect(_on_light_connected)
        light_device.services_discovered.connect(_on_services_found)
        light_device.characteristic_written.connect(_on_command_sent)
        light_device.connect_async()

func _on_light_connected():
    print("Light bulb connected")
    light_device.discover_services()

func _on_services_found(services: Array):
    print("Service discovery complete, ready to control light")

func set_light_color(red: int, green: int, blue: int):
    # Construct color command (example format)
    var command = PackedByteArray([0x01, red, green, blue])
    light_device.write_characteristic(LIGHT_SERVICE, LIGHT_CONTROL, command, false)

func turn_on():
    var command = PackedByteArray([0x02, 0x01])
    light_device.write_characteristic(LIGHT_SERVICE, LIGHT_CONTROL, command, false)

func turn_off():
    var command = PackedByteArray([0x02, 0x00])
    light_device.write_characteristic(LIGHT_SERVICE, LIGHT_CONTROL, command, false)

func _on_command_sent(char_uuid: String):
    print("Command sent")

# Called from UI
func _on_red_button_pressed():
    set_light_color(255, 0, 0)

func _on_green_button_pressed():
    set_light_color(0, 255, 0)

func _on_blue_button_pressed():
    set_light_color(0, 0, 255)
```

---

## Debug Mode

GDBLE provides optional debug mode to help troubleshoot issues.

### Enable Debug Mode

```gdscript
# Enable debug mode - show detailed logs
bluetooth_manager.set_debug_mode(true)
```


### Debug Output Example

When debug mode is enabled, you'll see detailed internal logs:

```
[BLE Info] Starting Bluetooth adapter initialization
[BLE Debug] Checking initialization state
[BLE Debug] Creating Tokio runtime manager
[BLE Debug] Acquiring Bluetooth adapter
[BLE Info] Bluetooth adapter acquired successfully
[BLE Info] Starting BLE device scan for 10 seconds
[BLE Debug] Spawning scan task asynchronously
[BLE Info] Discovered device: MyDevice (XX:XX:XX:XX:XX:XX), RSSI: -45
```

### Disable Debug Mode

```gdscript
# Disable debug mode (default) - keep output clean
bluetooth_manager.set_debug_mode(false)
```

### Conditional Debugging

```gdscript
# Enable only in debug builds
bluetooth_manager.set_debug_mode(OS.is_debug_build())
```

---

## FAQ

### Q: Why can't I find any devices?

**A:** Please check:
1. Ensure Bluetooth adapter is enabled
2. Ensure target device is in advertising state
3. Check if device is within range (usually within 10 meters)
4. Enable debug mode to view detailed logs

### Q: What if device connection fails?

**A:** Possible reasons:
1. Device is already connected by another application
2. Device is out of range
3. Device battery is low
4. Pairing required but not paired

### Q: How to handle device disconnection?

**A:** Listen to the `disconnected` signal:

```gdscript
device.disconnected.connect(_on_device_disconnected)

func _on_device_disconnected():
    print("Device disconnected, attempting reconnection...")
    # Implement reconnection logic
```

### Q: Write operation has no response?

**A:** Check:
1. Does characteristic support write (check `properties.write`)
2. Is data format correct
3. Do you need to use `with_response` parameter
4. Enable debug mode to view error messages

### Q: How to find correct service and characteristic UUIDs?

**A:** 
1. Check device documentation or specifications
2. Use debug mode to view all services
3. Use standard BLE services (e.g., heart rate, battery)
4. Use third-party BLE scanning tools (e.g., nRF Connect)

### Q: Does it support connecting to multiple devices simultaneously?

**A:** Yes, GDBLE supports managing multiple device connections:

```gdscript
var device1 = bluetooth_manager.connect_device(address1)
var device2 = bluetooth_manager.connect_device(address2)
```


### Q: How's the performance? Will it affect game frame rate?

**A:** GDBLE uses async architecture, all Bluetooth operations execute in background threads, won't block main thread or affect game frame rate.

---

## Data Format Description

### DeviceInfo Dictionary

Device information dictionary contains the following fields:

```gdscript
{
    "name": String,      # Device name (may be empty)
    "address": String,   # Device address (UUID or MAC)
    "rssi": int         # Signal strength (dBm)
}
```

### Service Dictionary

Service dictionary contains the following fields:

```gdscript
{
    "uuid": String,                    # Service UUID
    "characteristics": Array[Dictionary]  # Characteristics list
}
```

### Characteristic Dictionary

Characteristic dictionary contains the following fields:

```gdscript
{
    "uuid": String,        # Characteristic UUID
    "properties": {        # Characteristic properties
        "read": bool,                    # Can read
        "write": bool,                   # Can write (with response)
        "write_without_response": bool,  # Can write (without response)
        "notify": bool,                  # Supports notifications
        "indicate": bool                 # Supports indications
    }
}
```

---

## Best Practices

### 1. Error Handling

Always handle error cases:

```gdscript
bluetooth_manager.adapter_initialized.connect(func(success, error):
    if not success:
        push_error("Bluetooth initialization failed: " + error)
        # Show error message to user
)

device.connection_failed.connect(func(error):
    push_error("Connection failed: " + error)
    # Retry or notify user
)
```

### 2. Resource Cleanup

Clean up resources when node is destroyed:

```gdscript
func _exit_tree():
    if device and device.is_connected():
        device.disconnect()
    if bluetooth_manager:
        bluetooth_manager.stop_scan()
```

### 3. Timeout Handling

Set timeout for long-running operations:

```gdscript
var connection_timeout = 10.0
var timeout_timer: Timer

func connect_with_timeout(address: String):
    timeout_timer = Timer.new()
    add_child(timeout_timer)
    timeout_timer.timeout.connect(_on_connection_timeout)
    timeout_timer.start(connection_timeout)
    
    device = bluetooth_manager.connect_device(address)
    device.connected.connect(_on_connected)
    device.connect_async()

func _on_connected():
    timeout_timer.stop()
    print("Connection successful")

func _on_connection_timeout():
    print("Connection timeout")
    if device:
        device.disconnect()
```


### 4. State Management

Maintain clear connection state:

```gdscript
enum DeviceState {
    DISCONNECTED,
    CONNECTING,
    CONNECTED,
    DISCOVERING_SERVICES,
    READY
}

var device_state = DeviceState.DISCONNECTED

func connect_device(address: String):
    device_state = DeviceState.CONNECTING
    device = bluetooth_manager.connect_device(address)
    # ...

func _on_connected():
    device_state = DeviceState.CONNECTED
    device.discover_services()
    device_state = DeviceState.DISCOVERING_SERVICES

func _on_services_discovered(services: Array):
    device_state = DeviceState.READY
    # Now ready for read/write operations
```

### 5. Data Validation

Validate received data:

```gdscript
func _on_characteristic_read(char_uuid: String, data: PackedByteArray):
    if data.size() == 0:
        push_warning("Received empty data")
        return
    
    if data.size() < expected_size:
        push_warning("Data length insufficient")
        return
    
    # Process data
    process_data(data)
```

---

## Performance Optimization Tips

### 1. Scan Optimization

```gdscript
# Use appropriate scan time
bluetooth_manager.start_scan(5.0)  # 5 seconds is usually enough

# Stop scanning immediately after finding target device
func _on_device_discovered(info: Dictionary):
    if info.get("name") == target_name:
        bluetooth_manager.stop_scan()
```

### 2. Write Optimization

```gdscript
# For data that doesn't need confirmation, use write without response
device.write_characteristic(service, char, data, false)  # Faster

# For important data, use write with response
device.write_characteristic(service, char, data, true)   # More reliable
```

### 3. Notification Optimization

```gdscript
# Only subscribe to needed characteristics
device.subscribe_characteristic(service, char)

# Unsubscribe when no longer needed
device.unsubscribe_characteristic(service, char)
```

---

## Troubleshooting

### Enable Detailed Logging

```gdscript
bluetooth_manager.set_debug_mode(true)
```

### Check Adapter Status

```gdscript
if not bluetooth_manager.is_initialized():
    print("Adapter not initialized")
```

### Verify Device Connection

```gdscript
if device and device.is_connected():
    print("Device connected")
else:
    print("Device not connected")
```

### View Discovered Services

```gdscript
func _on_services_discovered(services: Array):
    print("Discovered services:")
    for service in services:
        print("  Service: ", service.get("uuid"))
        for char in service.get("characteristics", []):
            print("    Characteristic: ", char.get("uuid"))
            print("    Properties: ", char.get("properties"))
```

---

## Contributing

Contributions, issue reports, and suggestions are welcome!

- Report Issues: [GitHub Issues](https://github.com/Fantety/GodotBLE/issues)
- Submit Code: [Pull Requests](https://github.com/Fantety/GodotBLE/pulls)

---

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) file for details.

---

## Acknowledgments

- [btleplug](https://github.com/deviceplug/btleplug) - Excellent Rust BLE library
- [godot-rust](https://github.com/godot-rust/gdext) - Godot Rust bindings
- Godot community for support and feedback

---

## Contact

- GitHub: [@Fantety](https://github.com/Fantety)
- Project Homepage: [GodotBLE](https://github.com/Fantety/GodotBLE)

---

<div align="center">
  <p>If this project helps you, please give it a ⭐️ Star!</p>
  <p>Made with ❤️ for Godot Community</p>
</div>
