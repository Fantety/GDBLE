# ADR 0005: Session epochs, notification preparation, and event reservations

- Status: Accepted
- Date: 2026-10-16
- Extends: ADR 0001, 0002, and 0004

## Decision

Update Godot caches before preparing signal targets. Hold the Manager's `BaseMut` guard throughout device signal dispatch so synchronous callbacks may read or mutate the Manager, operate on other devices, and call `queue_free()`. Device events still precede Manager forwarding. Local rejection remains synchronous. A retained device keeps its closed client and operation ID sequence after Manager destruction; subsequent calls synchronously fail with `CANCELLED`.

Connection epochs are independent of request IDs. A system disconnect during a local disconnect records physical confirmation; it does not cancel that request or a queued reconnect. Already-disconnected echoes are ignored. After a local disconnect, suspicious events during reconnect are held until the connect future completes; suspicious events on a restored connection trigger a bounded backend state query. Query results are checked against their connection epoch. A failed query invalidates the session and schedules bounded cleanup. There is no elapsed-time suppression window.

Adapter attachment and availability are separate state. Initialization queries real availability. Power-off/unknown invalidates initialization, scans, and devices with a total two-second cleanup budget but retains a valid adapter listener. Power-on sends a nonterminal `initialize` progress event (String `data = "powered_on"`); applications explicitly initialize again. Advertisement changes enter a separate bounded data inbox; snapshots run in independent tasks with the normal operation deadline. RSSI, manufacturer data, service data, and service UUID updates merge into snapshots before deduplication. Service modification clears GATT/subscription caches and reports `SERVICES_CHANGED`.

On Android, btleplug 0.12 always returns `Unknown` from its state query. The v2 plugin supplies a real BluetoothAdapter query and power broadcasts through JNI, with a bounded 32-entry bridge stream. Lag invalidates availability before consuming newer states. No Godot Gd handle is retained by this bridge; Activity recreation refreshes the Java context reference.

The sole device notification pump must report ready before backend subscribe starts. A complete GATT route is registered before that call. Notifications received during preparation are buffered per device, up to 64 values and 64 KiB, and delivered after subscription success. A failed attempt removes its route and buffered data; queued packets retain an attempt ID so they cannot cross into a replacement route. Pump epochs also invalidate data already queued toward Godot.

Any notification overflow stops that pump, clears routes and undelivered data, and attempts backend unsubscribe within a two-second budget. The connection remains. Report `EVENT_OVERFLOW` as an unsolicited notification failure and through the legacy `operation_failed` projection. Resubscription creates a new pump and actually calls the backend.

Before accepting a request, reserve two real outbound channel permits for its start and terminal events. Partial reservation or command admission failure releases all permits and returns synchronous `QUEUE_FULL`. Rejections that need no started event release that unused permit. Active adapter, scan, connection, and notification sessions also reserve exceptional state slots before activation. The coordinator never awaits outbound channel capacity.

Commands retain capacity 64, outbound events 1024, and Godot drains at most 256 queued events per frame. Internal control and high-rate data have separate channels, each bounded at 256. Pending scan updates are coalesced by device, evict the oldest update at capacity, and report overflow. Each lifecycle slot may hold one pending authoritative state when its next permit is unavailable; repeated unsolicited state changes coalesce. This bounded state mailbox is not a request completion queue. Request outcomes never coalesce. Session slots are retired after pending delivery; shutdown clears all reservations.

## Public contract

No public BLE method parameters, legacy signals, or event dictionary keys change. Non-request failures have `operation_id = 0`, `terminal = false`. `error.details.state` describes adapter unavailability; `error.details.source` identifies overflow. New codes are `ADAPTER_UNAVAILABLE`, `EVENT_OVERFLOW`, and `SERVICES_CHANGED`. Availability hints use existing `initialize` events; they do not represent a successful initialization request.

## Verification and release

The 2026-10-18 review tightens two boundaries without changing public method parameters: scan stop reads the live cancellation registry and cancels already queued starts, so stale main-thread events cannot select an ended scan; unsubscribe validates the full GATT key, with inactive keys succeeding as `already_unsubscribed` without touching backend state or another route. See [API_CONTRACT](../API_CONTRACT.md) for payload shapes and request timing.

Formal fake-backend tests cover echoes, timeouts, advertisement categories, power cycles, service invalidation, subscription preparation, overflow, and stopped-consumer pressure. The `test-harness` Cargo feature injects a fake backend only for Godot success-path tests. Dedicated Cargo target directories isolate fixtures from production builds; final-addon smoke tests never compile or replace binaries.

Native libraries embed version, source commit, canonical UTF-8/LF source digest, target, toolchain, and test-feature status. The release manifest binds these records to artifact hashes. A later release commit is accepted only if its source digest still matches. Missing AARs, stale libraries, mismatched checksums, and test builds fail release validation. Hardware BLE evidence remains a separate release requirement.
