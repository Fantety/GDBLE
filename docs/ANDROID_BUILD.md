# Android build and packaging

GDBLE 0.6.0 uses the Godot Android v2 plugin architecture. Android exports require Godot 4.2+, Gradle builds, JDK 17, compileSdk 34, minSdk 23, and two AARs.

## Prerequisites

- JDK 17
- Android SDK platform 34
- Android NDK
- Rust targets `aarch64-linux-android` and `x86_64-linux-android`
- `cargo install cargo-ndk`
- `ANDROID_NDK_HOME` set to the NDK directory
- Optional `GDBLE_GRADLE` pointing to an installed Gradle 8.9+ executable; otherwise the locked btleplug wrapper is used. Prefer Gradle 8.9 for reproducible AGP 8.7.3 builds.

## Build

```powershell
.\scripts\build-android.ps1
```

```bash
./scripts/build-android.sh
```

The scripts use `cargo metadata --locked` to locate the Java project belonging to btleplug 0.12.0. They build:

- `addons/gdble/android/gdble-release.aar`
- `addons/gdble/android/btleplug-release.aar`

The GDBLE AAR contains the Kotlin `GodotPlugin`, ARM64/x86_64 `libgdble.so`, and the generated `.gdextension` asset. The btleplug Java bridge remains a separate AAR.
`addons/gdble/gdble.gdap` exports the GDBLE AAR and its local btleplug AAR dependency together.

## Godot export

1. Copy the complete canonical `addons/gdble` directory into the project.
2. Install the Godot Android build template and enable Gradle Build.
3. Set min SDK 23 and target SDK 34.
4. Enable ARM64; optionally enable x86_64 for emulator loading tests.
5. Gate initialization on the complete runtime permission result. Both demos use the same helper:

```gdscript
var startup = preload("res://addons/gdble/permission_startup.gd").new()
add_child(startup)
startup.permission_denied.connect(func(missing):
    print("BLE remains uninitialized; enable permissions in Settings: ", missing))
startup.start(bluetooth_manager)
```

The native plugin never opens permission dialogs automatically. This opt-in application helper checks granted permissions first, requests once if needed, and rereads the complete set after `SceneTree.on_request_permissions_result`. It uses `GDBLE.getAndroidApiLevel()` instead of inferring API level from a human-readable OS version. Partial or permanent denial leaves the Manager uninitialized and emits `permission_denied`. Returning from Settings or the background checks permissions again without opening another dialog. Applications decide whether to offer an explicit retry UI.

## Permissions

| Android version | Permissions |
| --- | --- |
| API 23–30 | BLUETOOTH and BLUETOOTH_ADMIN in the manifest; ACCESS_FINE_LOCATION at runtime |
| API 31+ | BLUETOOTH_SCAN, BLUETOOTH_CONNECT |

No-adapter, denied-permission, or JNI initialization failures must surface through `ble_event` and must not panic.

The permission gate avoids initializing after denial and reports through its own `permission_denied` signal. Apps bypassing it receive backend failures through `ble_event`. On older Android versions, scanning may additionally require the system location service to be enabled.

btleplug 0.12 reports an unknown adapter state on Android. The GDBLE v2 plugin therefore queries the real `BluetoothAdapter` through JNI and forwards protected Bluetooth power broadcasts into a bounded state stream. Missing permission and turning-on/off states remain unavailable. Activity recreation updates the context bridge; returning to the foreground refreshes availability without reconnecting devices.

## Validation

Inspect both AARs before release:

- GDBLE manifest contains `org.godotengine.plugin.v2.GDBLE` metadata.
- `jni/arm64-v8a/libgdble.so` and `jni/x86_64/libgdble.so` are present.
- The btleplug AAR contains `com/nonpolynomial/btleplug/android/impl` classes.
- JNI initialization is idempotent.

Release validation requires API 23–30 and API 31+ ARM64 devices for scan/connect/discover/read/write/notify/remote-disconnect, plus an x86_64 emulator loading test.

Run `python scripts/test-godot.py --mode permissions` for already-granted, first-grant, partial-denial, permanent-denial, and foreground transition cases. These deterministic tests and AAR structure checks do not replace the real-device BLE loop. Record missing device evidence as incomplete.
