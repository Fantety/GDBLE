# GDBLE Domain Context

GDBLE exposes Bluetooth Low Energy operations to Godot while keeping all asynchronous ownership outside Godot objects. These terms are the shared language for code, tests, issues, and ADRs.

## Domain terms

- **Godot Adapter**: `BluetoothManager` or `BleDevice`. It submits commands, maintains main-thread read caches, projects structured events to legacy signals, and never owns a system BLE object or Tokio task.
- **BleCore**: the worker-thread owner of the Tokio runtime, backend, command/event channels, task set, sessions, cancellation, and shutdown order.
- **Adapter Session**: the single initialized system adapter and its one long-lived adapter event stream. It routes discovery updates and remote disconnects.
- **Scan Session**: one scan operation with an operation ID, generation, cancellation token, and `Idle → Starting → Scanning → Stopping` lifecycle.
- **Device Session**: the ordered command queue and runtime state for one normalized device address. A remote disconnect invalidates in-flight and queued work.
- **Operation**: one command-triggered request. Positive operation IDs receive exactly one terminal `succeeded`, `failed`, or `cancelled` event.
- **Subscription**: a device-local route keyed by `(service_uuid, characteristic_uuid)`. A device owns at most one notification pump.
- **Notification Pump**: the sole btleplug notification stream for a connected device. It routes characteristic UUID notifications back to the full subscription key.
- **Backend**: the internal `BleBackend` interface. `BtleplugBackend` is production infrastructure and `FakeBackend` is the deterministic test adapter.
- **Command/Event Seam**: bounded channels between Godot adapters and BleCore. Commands have capacity 64, events capacity 1024, and Godot processes at most 256 events per frame.
- **Display Address**: the address text shown to GDScript. It is separate from the trimmed, lowercase internal `DeviceId` identity.

## Invariants

1. Background threads never retain or invoke `Gd<T>`.
2. System BLE state has one owner: BleCore.
3. Adapter events and notification streams are singular per adapter/device.
4. UUIDs are parsed and emitted in lowercase hyphenated form.
5. A failed connect never enters the connected cache; a remote disconnect clears connection and subscription state immediately.
6. Legacy 0.5.x methods and signals are projections of the structured event protocol through at least 1.0.
