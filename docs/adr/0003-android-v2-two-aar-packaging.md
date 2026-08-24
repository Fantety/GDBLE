# ADR 0003: Godot Android v2 plugin with two AARs

- Status: Accepted
- Date: 2026-08-24

## Context

Raw Android `.so` files do not initialize btleplug's Java bridge and do not provide a complete Godot export unit. Copying upstream Java sources into this repository would drift from the locked Rust dependency.

## Decision

Ship Android as two AARs:

1. `gdble-release.aar` contains the Kotlin `GodotPlugin`, generated `.gdextension` asset, and ARM64/x86_64 Rust libraries.
2. `btleplug-release.aar` is built from the Java project located through locked Cargo metadata for btleplug 0.12.0.

The Kotlin plugin loads `gdble` and invokes explicit JNI initialization. Rust stores the result in `OnceLock<Result<(), String>>`; later initialization failures become structured `ANDROID_NOT_INITIALIZED` events. The plugin declares permissions but never opens permission dialogs.

## Consequences

- Android packaging matches Godot 4.2+ plugin v2 and avoids a fragile fat AAR.
- The Java bridge version cannot silently diverge from Cargo.lock.
- Applications remain responsible for calling `OS.request_permissions()` before Bluetooth initialization.
