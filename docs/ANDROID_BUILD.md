# Android build and packaging

GDBLE 0.6.0 uses the Godot Android v2 plugin architecture. Android exports require Godot 4.2+, Gradle builds, JDK 17, compileSdk 34, minSdk 23, and two AARs.

## Prerequisites

- JDK 17
- Android SDK platform 34
- Android NDK
- Rust targets `aarch64-linux-android` and `x86_64-linux-android`
- `cargo install cargo-ndk`
- `ANDROID_NDK_HOME` set to the NDK directory

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
5. Request runtime permissions before calling `BluetoothManager.initialize()`:

```gdscript
if OS.get_name() == "Android":
    OS.request_permissions()
bluetooth_manager.initialize()
```

The plugin never opens permission dialogs automatically.

## Permissions

| Android version | Permissions |
| --- | --- |
| API 23–30 | BLUETOOTH, BLUETOOTH_ADMIN, ACCESS_FINE_LOCATION |
| API 31+ | BLUETOOTH_SCAN, BLUETOOTH_CONNECT |

No-adapter, denied-permission, or JNI initialization failures must surface through `ble_event` and must not panic.

## Validation

Inspect both AARs before release:

- GDBLE manifest contains `org.godotengine.plugin.v2.GDBLE` metadata.
- `jni/arm64-v8a/libgdble.so` and `jni/x86_64/libgdble.so` are present.
- The btleplug AAR contains `com/nonpolynomial/btleplug/android/impl` classes.
- JNI initialization is idempotent.

Release validation requires API 23–30 and API 31+ ARM64 devices for scan/connect/discover/read/write/notify/remote-disconnect, plus an x86_64 emulator loading test.
