#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_dir="$repo_root/addons/gdble"
destination="$repo_root/demo/addons/gdble"

case "$destination" in
    "$repo_root"/demo/addons/*) ;;
    *)
        echo "Refusing to replace a directory outside demo/addons: $destination" >&2
        exit 1
        ;;
esac

rm -rf -- "$destination"
mkdir -p "$(dirname "$destination")"
cp -R "$source_dir" "$destination"
