# GDBLE 0.6 API contract

The public API is the Godot `BluetoothManager` / `BleDevice` API. Rust core types are internal implementation details. Existing 0.5.x method parameters and signals remain compatible. New code should use `get_or_create_device()` to obtain a handle and `connect_async()` to connect; the historical `connect_device()` name remains a factory alias.

## Requests and event timing

Connect signals before submitting work. Accepted requests complete asynchronously. Invalid arguments, closed clients, and admission failures can emit a terminal failure synchronously before the method returns its positive ID. A rejected request does not alter connection state. An ID is unique within its owning Manager, not across Managers or standalone devices. An accepted request may finish without a `started` event, for example an idempotent operation.

Each positive ID has one terminal event while the Manager remains alive and drains events. `stop_scan()` returns an existing scan ID, not a new request ID. It cancels all unfinished scan submissions present at the call, including queued starts; the returned ID is the oldest such request, or `0` if there is none. Later scan submissions are independent. Old main-thread events cannot redirect cancellation to an already finished scan.

For a pending-request dictionary, use deferred signal delivery so the returned ID is stored before the handler runs. This pattern applies to all request methods, including read, write, subscription, connection, and initialization:

```gdscript
var pending := {}

func watch_device(device: BleDevice) -> void:
    device.ble_event.connect(_on_device_event, CONNECT_DEFERRED)

func read_value(device: BleDevice, service: String, characteristic: String) -> void:
    var id := device.read_characteristic(service, characteristic)
    pending[id] = {"operation": "read", "address": device.get_address()}

func _on_device_event(event: Dictionary) -> void:
    if event.terminal and pending.has(event.operation_id):
        var request: Dictionary = pending[event.operation_id]
        pending.erase(event.operation_id)
        print(request, " -> ", event.phase, " ", event.error)
```

The example assumes devices from one Manager. With several Managers, use separate request maps or include the Manager identity. Deferred callbacks observe the latest cache, which may have advanced beyond the event; use the event payload for that request's result. Direct callbacks observe the cache projection applied immediately before signal emission.

Device-local rejection events are emitted only by the device. Accepted device events are emitted by the device and then forwarded through the Manager. Listening to both without filtering duplicates accepted outcomes. For one event stream per operation, submit device work through `BleDevice`, consume its events, and consume only `initialize`, `scan`, and `resolve_device` from the Manager. If using the compatibility `disconnect_device()` method, also handle its Manager-local rejection events. The demo controller implements deferred routing and request tracking.

## Event payloads

Every event has `operation`, `phase`, `operation_id`, `terminal`, `device_address`, `service_uuid`, `characteristic_uuid`, `data`, and `error`. `operation` and `phase` are `StringName`; reason payloads are `String`, never a dictionary with a `reason` member.

| Operation / phase | `data` |
| --- | --- |
| `initialize / succeeded` | Adapter dictionary: `name: String`, `address: String or null` |
| `initialize / progress` after power-on | String `"powered_on"`; this is a hint to explicitly initialize again |
| `scan / progress` | DeviceInfo dictionary plus `kind: StringName` (`discovered` or `updated`) |
| `scan / succeeded` | String `"stopped"` or `"timeout"` |
| `connect / succeeded` | `null`, or String `"already_connected"` |
| `disconnect / succeeded` | `null`, or String `"already_disconnected"` |
| `disconnect / received` | String `"remote"` |
| `discover_services / succeeded` | Array of service dictionaries (`uuid`, `characteristics`) |
| `read / succeeded`, `notification / received` | `PackedByteArray` |
| `write / succeeded` | `null` |
| `subscribe / succeeded` | `null`, or String `"already_subscribed"` |
| `unsubscribe / succeeded` | `null`, or String `"already_unsubscribed"` |
| `resolve_device / failed` | `null`; factory returns `null`. Successful factory lookup returns a handle without an operation event |
| `started`, `failed`, `cancelled` | `null`; inspect `error` for failure details |

Device identities are trimmed and lowercase internally; display addresses can preserve platform casing. Valid UUIDs use lowercase hyphenated form. Invalid-argument events can retain the original invalid UUID text for diagnostics. Use `(device_address, service_uuid, characteristic_uuid)` when correlating GATT values.

Subscriptions are keyed by the full service/characteristic pair. On a connected session whose GATT state has not been invalidated, an unsubscribe for an inactive pair is an idempotent success and never reaches the backend, including when another service has an active subscription with the same characteristic UUID. An unsuccessful unsubscribe of an active pair preserves it. Simultaneous subscriptions to duplicate characteristic UUIDs across services remain unsupported because the backend notification payload lacks the service identity.

## Errors

`error` always contains `code: String`, `message: String`, `retryable: bool`, and `details: Dictionary`. Successful events use an empty code/message/details and `retryable = false`. Branch on codes, not human-readable messages.

| Codes | Meaning |
| --- | --- |
| `INVALID_ARGUMENT`, `INVALID_UUID` | Invalid request input |
| `NOT_INITIALIZED`, `ANDROID_NOT_INITIALIZED` | Manager or Android bridge is not initialized |
| `ADAPTER_NOT_FOUND`, `ADAPTER_UNAVAILABLE` | Missing/unavailable adapter; unavailable state is in `details.state` |
| `DEVICE_NOT_FOUND`, `NOT_CONNECTED`, `CHARACTERISTIC_NOT_FOUND` | Required device, connection, or GATT object is unavailable |
| `BUSY`, `QUEUE_FULL` | Conflicting operation or exhausted admission capacity |
| `AMBIGUOUS_CHARACTERISTIC` | A characteristic UUID would have two service subscription routes |
| `SERVICES_CHANGED` | Rediscover services before further GATT work |
| `EVENT_OVERFLOW` | Event data exceeded capacity; `details.source` identifies the buffer |
| `INITIALIZATION_FAILED`, `SCAN_FAILED`, `CONNECTION_FAILED` | Backend initialization, scan, or connection failure |
| `SERVICE_DISCOVERY_FAILED`, `READ_FAILED`, `WRITE_FAILED` | Backend GATT operation failure |
| `SUBSCRIBE_FAILED`, `UNSUBSCRIBE_FAILED` | Backend subscription operation failure |
| `OPERATION_FAILED`, `TIMEOUT`, `INTERNAL_ERROR` | Other backend failure, operation deadline, or caught internal failure |
| `CANCELLED` | Request was cancelled or its owning core has closed |

`retryable` is true for `TIMEOUT`, `CONNECTION_FAILED`, `OPERATION_FAILED`, `QUEUE_FULL`, `ADAPTER_UNAVAILABLE`, and `EVENT_OVERFLOW`. It is a hint, not an automatic retry policy: restore availability or subscriptions as needed, avoid retrying writes blindly, and use bounded application retries. Unsolicited state/errors have ID `0` and `terminal = false`.

## Ownership and cache lifetime

Add the Manager to the scene tree and keep it processing while awaiting results. `BleDevice` is a reference-counted handle owned by that Manager; constructing it directly produces an unattached handle whose operations fail. Manager destruction cancels work, clears device caches, and leaves retained device handles returning `CANCELLED`. Disconnecting a device does not release its handle from the Manager.

Discovery is deduplicated over the adapter lifetime, not reset on each scan. `get_discovered_devices()` returns the accumulated cache until adapter invalidation or Manager destruction; a repeated unchanged advertisement need not emit another discovery event. There is currently no per-device forget API or automatic eviction of the Manager/core identity caches. Bounded channels do not imply bounded lifetime discovery storage. Applications needing to discard the entire inventory should end their BLE session, release its device references, and destroy the Manager before creating a new session.

The debug-mode methods retain a process-wide compatibility flag. They do not enable an additional native tracing stream; application logging is driven by `ble_event`.
