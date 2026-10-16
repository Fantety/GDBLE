# Build scripts

`addons/gdble` is the canonical distribution directory.

- `build.ps1` / `build.sh`: build the current desktop host, optionally build Android, then generate `demo/addons/gdble`.
- `scripts/build-android.ps1` / `.sh`: build ARM64 and x86_64 Rust libraries, the GDBLE Android v2 AAR, and the separate locked btleplug bridge AAR.
- `scripts/sync-demo-addon.ps1` / `.sh`: replace the generated Demo addon with the canonical addon.
- `scripts/check-lifecycle.py`: check the asynchronous ownership rules and per-frame event budget using Python 3.
- `scripts/validate-android-aar.py [directory]`: validate native libraries, the extension asset, plugin metadata, and Java classes in both AARs using Python 3. The default directory is `addons/gdble/android`.
- `scripts/release-notes.py <tag>`: require a matching Cargo version and a nonempty UTF-8 file at `release-notes/<tag>.md`. Requires Python 3.11+ for the standard-library TOML parser.
- `scripts/test-build-failures.ps1`: inject desktop Cargo, metadata, Android Rust, bridge Gradle, and plugin Gradle failures in isolated fixtures under `target/`. Verify failure propagation, working-directory restoration, and preservation of existing distribution artifacts.

All Cargo commands use `--locked`. Android builds require `cargo-ndk`, JDK 17, SDK 34, and `ANDROID_NDK_HOME`.

Desktop and Android scripts use `target/desktop-build` and `target/android-build`. Tests use independent directories (`target/repair-build`, `target/godot-source-build`, `target/godot-harness-build`); never share a Cargo target directory with copied review fixtures. The optional `test-harness` feature is exclusively for integration tests and must not be used to build release artifacts.

`scripts/artifact_provenance.py generate|validate` records/checks embedded build information and the complete addon hash inventory. `scripts/test-godot.py --mode artifact` validates the actual packaged library without compiling; `--mode source`, `--mode harness`, and `--mode permissions` cover fresh source, callback success paths, and permission transitions. Pass `--godot /absolute/path/to/godot` when it is not on PATH.

PowerShell scripts check each native build command's exit code before copying its output. Both Android scripts resolve Cargo metadata from the repository root, so callers may start them from another directory.

Desktop output names:

- `libgdble.windows.x86_64.dll`
- `libgdble.linux.x86_64.so`
- `libgdble.macos.x86_64.dylib`
- `libgdble.macos.arm64.dylib`

Android is released only as:

- `android/gdble-release.aar`
- `android/btleplug-release.aar`

Raw Android `.so` files are generated inside the GDBLE AAR build directory and are not release files.

For a fresh Demo checkout, build and sync the addon first, then run the smoke test:

```bash
godot --headless --path demo --script smoke_test.gd
```

The test explicitly loads the `.gdextension` and resolves its classes through `ClassDB`, so it does not depend on an editor-generated `.godot/extension_list.cfg`. It exits with an error if loading fails or the checks do not finish within 10 seconds. Keep the generated `.godot` directory out of version control. CI runs the test on Godot 4.2.2 and 4.6.1.
