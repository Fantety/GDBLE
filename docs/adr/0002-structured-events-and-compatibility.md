# ADR 0002: Structured events with 0.5.x compatibility projection

- Status: Accepted
- Date: 2026-08-24

## Context

The old signal set omitted operation identity, complete GATT context, stable error codes, and a consistent terminal contract. Existing Godot projects still depend on those signal names and argument order.

## Decision

Add `ble_event(event: Dictionary)` to both public classes. Every event has fixed operation, phase, operation ID, terminal, device/service/characteristic context, data, and structured error fields. Asynchronous methods now return an ignorable positive operation ID. Existing signals remain unchanged and are projected from the same event on the Godot main thread.

`connect_device(address)` remains a handle factory and compatibility alias for `get_or_create_device(address)`. Only `BleDevice.connect_async()` performs a connection.

## Consequences

- New callers can correlate requests and terminal outcomes without parsing localized text.
- Old projects continue to run without changing parameters or signal handlers.
- Compatibility behavior is centralized instead of duplicated across background tasks.
