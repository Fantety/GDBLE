use godot::prelude::*;

// Module declarations
mod backend;
mod ble_characteristic;
mod ble_device;
mod ble_service;
mod bluetooth_manager;
mod core;
mod godot_event;
mod types;

#[cfg(target_os = "android")]
mod android;

// Re-export main classes for easier access
pub use ble_device::BleDevice;
pub use bluetooth_manager::BluetoothManager;

/// GDExtension entry point
///
/// This struct serves as the entry point for the Godot extension.
/// All classes marked with #[derive(GodotClass)] are automatically
/// registered when the extension is loaded.
struct GdBle;

#[gdextension]
unsafe impl ExtensionLibrary for GdBle {}
