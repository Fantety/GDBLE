#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
addon="$repo_root/addons/gdble"
host_os="$(uname -s)"
host_arch="$(uname -m)"

cd "$repo_root"
cargo build --release --locked

case "$host_os/$host_arch" in
    Linux/x86_64)
        cp target/release/libgdble.so "$addon/libgdble.linux.x86_64.so"
        ;;
    Darwin/x86_64)
        cp target/release/libgdble.dylib "$addon/libgdble.macos.x86_64.dylib"
        ;;
    Darwin/arm64)
        cp target/release/libgdble.dylib "$addon/libgdble.macos.arm64.dylib"
        ;;
    *)
        echo "Unsupported host platform: $host_os/$host_arch" >&2
        exit 1
        ;;
esac

if [[ -n "${ANDROID_NDK_HOME:-}" ]] && cargo ndk --version >/dev/null 2>&1; then
    "$repo_root/scripts/build-android.sh"
else
    echo "Android build skipped (ANDROID_NDK_HOME or cargo-ndk unavailable)."
fi

"$repo_root/scripts/sync-demo-addon.sh"
