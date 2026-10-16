# GDBLE Domain Context

GDBLE exposes Bluetooth Low Energy operations to Godot while keeping all asynchronous ownership outside Godot objects. These terms are the shared language for code, tests, issues, and ADRs.

## Domain terms

- **Godot Adapter**: `BluetoothManager` or `BleDevice`. It submits commands, maintains main-thread read caches, projects structured events to legacy signals, and never owns a system BLE object or Tokio task.
- **BleCore**: the worker-thread owner of the Tokio runtime, backend, command/event channels, task set, sessions, cancellation, and shutdown order.
- **Adapter Session**: the single initialized system adapter and its one long-lived adapter event stream. It routes discovery updates and remote disconnects.
- **Scan Session**: one scan operation with an operation ID, generation, cancellation token, and `Idle → Starting → Scanning → Stopping` lifecycle.
- **Device Session**: the ordered command queue and runtime state for one normalized device address. A remote disconnect invalidates in-flight and queued work.
- **Operation**: one command-triggered request. Positive operation IDs receive exactly one terminal `succeeded`, `failed`, or `cancelled` event while the owning Manager is alive and draining events. Destruction cancels work without guaranteeing remaining signal delivery.
- **Subscription**: a device-local route keyed by `(service_uuid, characteristic_uuid)`. A device owns at most one notification pump.
- **Notification Pump**: the sole btleplug notification stream for a connected device. It routes characteristic UUID notifications back to the full subscription key.
- **Backend**: the internal `BleBackend` interface. `BtleplugBackend` is production infrastructure and `FakeBackend` is the deterministic test adapter.
- **Command/Event Seam**: bounded channels between Godot adapters and BleCore. Commands have capacity 64, events capacity 1024, and Godot processes at most 256 queued events per frame. Local rejections are returned directly as `Submission` errors and emitted synchronously by the initiating Godot object; they never compete for event capacity. Scan cancellation uses registered tokens independently of command capacity.
- **State Snapshot**: internal connection/adapter state attached by BleCore to an event. Cache projections apply it before any signals. Local rejections omit snapshots and cannot invent physical state; snapshots are not added to the public event dictionary.
- **Deadline**: 30 seconds per backend operation after dequeue, with timeout/panic converted to a terminal failure. Stream lifetimes are not timed out. Shutdown and adapter recovery each share a two-second best-effort cleanup budget; runtime shutdown adds 100 ms grace. Async cancellation cannot preempt blocking native code or guarantee physical rollback.
- **Display Address**: the address text shown to GDScript. It is separate from the trimmed, lowercase internal `DeviceId` identity.

## Invariants

1. Background threads never retain or invoke `Gd<T>`.
2. System BLE state has one owner: BleCore.
3. Adapter events and notification streams are singular per adapter/device.
4. UUIDs are parsed and emitted in lowercase hyphenated form.
5. Rejected requests preserve existing caches; connection/disconnection results reflect the core state. A remote disconnect clears connection, service, and subscription state before callbacks.
6. Legacy 0.5.x methods and signals are projections of the structured event protocol through at least 1.0.
7. A nonterminal scan error cannot finish the active scan. Adapter events and all notification values, errors, and endings are checked against their generation.
8. Adapter stream termination cancels sessions and performs bounded cleanup before allowing reinitialization. Notification stream termination is observable and resubscription can restart its pump.

## Module map

- `src/core.rs`: backend coordination, state machines, deadlines, and tracked task cleanup.
- `src/core/client.rs`: nonblocking submission, independent scan cancellation, runtime ownership.
- `src/core/protocol.rs`: IDs, structured events, scan identity, and internal state snapshots.
- `src/core/outbox.rs`, `src/core/inbox.rs`: admission reservations, bounded state mailboxes, and isolated high-rate data.
- `src/core/lifecycle.rs`, `src/core/notifications.rs`: connection epochs, adapter availability, snapshots, and notification preparation/recovery.
- `src/event_cache.rs`: pure main-thread cache projections, tested without Godot.
- `src/backend/test_support.rs`, `src/core/tests.rs`, `src/core/regression_tests.rs`: deterministic backend, lifecycle, pressure, and failure-injection tests.
- `release-notes/v<version>.md`: release body read from the tagged commit; see `docs/RELEASING.md`.

ADR 0005 refines session behavior: local disconnect echoes preserve queued reconnects; adapter attachment survives power changes; notification overflow invalidates the pump while preserving connection. Accepted requests reserve their started/terminal slots before enqueue. Unsolicited lifecycle changes may coalesce to the latest state under sustained pressure; request completions never coalesce. See `docs/adr/0005-session-epochs-and-event-reservations.md` for limits and recovery rules.
