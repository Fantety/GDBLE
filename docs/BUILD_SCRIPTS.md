# Build scripts

`addons/gdble` is the canonical distribution directory.

- `build.ps1` / `build.sh`: build the current desktop host, optionally build Android, then generate `demo/addons/gdble`.
- `scripts/build-android.ps1` / `.sh`: build ARM64 and x86_64 Rust libraries, the GDBLE Android v2 AAR, and the separate locked btleplug bridge AAR.
- `scripts/sync-demo-addon.ps1` / `.sh`: replace the generated Demo addon with the canonical addon.

All Cargo commands use `--locked`. Android builds require `cargo-ndk`, JDK 17, SDK 34, and `ANDROID_NDK_HOME`.

Desktop output names:

- `libgdble.windows.x86_64.dll`
- `libgdble.linux.x86_64.so`
- `libgdble.macos.x86_64.dylib`
- `libgdble.macos.arm64.dylib`

Android is released only as:

- `android/gdble-release.aar`
- `android/btleplug-release.aar`

Raw Android `.so` files are generated inside the GDBLE AAR build directory and are not release files.
