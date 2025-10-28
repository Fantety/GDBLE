use godot::prelude::*;

// Module declarations
mod types;
mod runtime;
mod bluetooth_manager;
mod bluetooth_scanner;
mod ble_characteristic;
mod ble_service;
mod ble_device;

// Re-export main classes for easier access
pub use bluetooth_manager::BluetoothManager;
pub use ble_device::BleDevice;

/// GDExtension entry point
/// 
/// This struct serves as the entry point for the Godot extension.
/// All classes marked with #[derive(GodotClass)] are automatically
/// registered when the extension is loaded.
struct GdBle;

#[gdextension]
unsafe impl ExtensionLibrary for GdBle {}
