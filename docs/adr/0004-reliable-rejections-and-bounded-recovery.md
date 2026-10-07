# ADR 0004: Reliable rejection delivery and bounded recovery

- Status: Accepted
- Date: 2026-10-07
- Extends: ADR 0001 and ADR 0002

## Context

Local failures competed with accepted work for the bounded event queue and could lose terminal events. Scan cancellation competed for command capacity. Cache updates inferred physical state from request failure, while unbounded backend waits and silent stream termination could strand operations.

## Decision

Return local failures through `Submission` and emit them synchronously on the calling Godot object. Keep accepted requests and their events asynchronous, with existing bounded channels. Device-local rejection signals are not aggregated by the Manager. Callers connect handlers before submitting; terminal delivery is guaranteed only while the owner is alive and processes events. Scan cancellation uses a registered cancellation token outside the command queue.

Attach internal authoritative state snapshots to core events and update caches before signals. Local rejections leave snapshots absent. Only a matching terminal scan event ends the active scan. Preserve the existing public event dictionary.

Guard backend operations with a 30-second timeout and catch Rust panics at the backend future/stream boundary. Track tasks in the core. Shutdown and adapter recovery abort old work, drain it, then attempt scan stop and concurrent device disconnects under one two-second cleanup budget. Runtime shutdown has a separate 100 ms grace period. These bounds govern cooperative asynchronous code; they cannot guarantee physical rollback or interrupt native code blocking a runtime thread.

Treat adapter and notification stream closure as observable unsolicited failures. Adapter loss invalidates sessions before reinitialization is accepted; resubscribing restarts an ended notification pump. Check generations for values, errors, and endings so old streams cannot affect a replacement session.

## Consequences

- Pressure cannot silently drop local terminal failures or scan cancellation requests.
- A synchronous rejection can arrive before the caller stores the returned operation ID; this timing is documented in both READMEs and release notes.
- Retry and shutdown behavior can be tested with a fault-injecting fake backend without radio hardware.
- Automatic reconnect remains outside this change. Applications choose when to initialize or subscribe again after a stream failure.
