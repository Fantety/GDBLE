# ADR 0001: Worker-owned asynchronous core

- Status: Accepted
- Date: 2026-08-24

## Context

The 0.5.x implementation split ownership across `RuntimeManager`, `BluetoothScanner`, `BluetoothManager`, and each `BleDevice`. It used unbounded channels, synchronous waits from Godot, detached tasks, and Godot handles behind cross-thread mutexes.

## Decision

Use a deep `BleCore` module on one worker thread. It owns the Tokio runtime, bounded command/event channels, `JoinSet`, cancellation tokens, one adapter event stream, scan state, and per-device sessions. `BluetoothManager` and `BleDevice` are main-thread adapters only. Production and tests share the `BleBackend` interface.

## Consequences

- Lifecycle and stale-result rules are local to one module and deterministic under `FakeBackend` tests.
- Godot calls return immediately and caches answer synchronous getters.
- Shutdown can cancel and join all tracked work before dropping the runtime.
- The core must maintain explicit state machines and bounded queues rather than relying on incidental backend state.
