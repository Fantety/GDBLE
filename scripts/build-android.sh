#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
metadata="$(cargo metadata --format-version 1 --locked)"
btleplug_manifest="$(printf '%s' "$metadata" | python3 -c '
import json, sys
packages = json.load(sys.stdin)["packages"]
match = next((p for p in packages if p["name"] == "btleplug" and p["version"] == "0.12.0"), None)
if match is None:
    raise SystemExit("btleplug 0.12.0 was not found in locked Cargo metadata")
print(match["manifest_path"])
')"
btleplug_root="$(dirname "$btleplug_manifest")"
bridge_project="$btleplug_root/src/droidplug/java"
generated_jni="$repo_root/android-plugin/plugin/build/generated/jniLibs"

cd "$repo_root"
cargo ndk -t arm64-v8a -t x86_64 -o "$generated_jni" build --release --locked
"$bridge_project/gradlew" -p "$bridge_project" assembleRelease
"$bridge_project/gradlew" -p "$repo_root/android-plugin" :plugin:assembleRelease

mkdir -p "$repo_root/addons/gdble/android"
cp "$repo_root/android-plugin/plugin/build/outputs/aar/plugin-release.aar" \
    "$repo_root/addons/gdble/android/gdble-release.aar"
bridge_aar="$(find "$bridge_project/build/outputs/aar" -maxdepth 1 -name '*-release.aar' -print -quit)"
test -n "$bridge_aar"
cp "$bridge_aar" "$repo_root/addons/gdble/android/btleplug-release.aar"
