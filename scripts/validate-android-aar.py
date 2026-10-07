"""Validate the two Android AARs without external archive or search tools."""

import argparse
import io
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from zipfile import BadZipFile, ZipFile


def require_entries(archive: ZipFile, entries: tuple[str, ...]) -> None:
    for entry in entries:
        if entry not in archive.namelist() or archive.getinfo(entry).file_size == 0:
            raise ValueError(f"{archive.filename}: missing or empty {entry}")


def validate_android_aars(directory: Path) -> None:
    with ZipFile(directory / "gdble-release.aar") as archive:
        require_entries(archive, (
            "jni/arm64-v8a/libgdble.so",
            "jni/x86_64/libgdble.so",
            "assets/addons/gdble/gdble.gdextension",
            "AndroidManifest.xml",
            "classes.jar",
        ))

        # Library AAR manifests are plain XML, unlike the binary XML in APKs.
        manifest = ET.fromstring(archive.read("AndroidManifest.xml"))
        android = "{http://schemas.android.com/apk/res/android}"
        if not any(
            entry.get(f"{android}name") == "org.godotengine.plugin.v2.GDBLE"
            and entry.get(f"{android}value") == "org.gdble.android.GDBLEPlugin"
            for entry in manifest.findall("./application/meta-data")
        ):
            raise ValueError("gdble-release.aar: missing Godot v2 plugin metadata")

        with ZipFile(io.BytesIO(archive.read("classes.jar"))) as classes:
            require_entries(classes, ("org/gdble/android/GDBLEPlugin.class",))

    with ZipFile(directory / "btleplug-release.aar") as archive:
        require_entries(archive, ("classes.jar",))
        with ZipFile(io.BytesIO(archive.read("classes.jar"))) as classes:
            if not any(
                entry.filename.startswith("com/nonpolynomial/btleplug/android/impl/")
                and entry.filename.endswith(".class")
                and entry.file_size > 0
                for entry in classes.infolist()
            ):
                raise ValueError("btleplug-release.aar: missing btleplug bridge classes")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "directory", nargs="?", type=Path,
        default=Path(__file__).resolve().parent.parent / "addons" / "gdble" / "android",
    )
    args = parser.parse_args()
    try:
        validate_android_aars(args.directory)
    except (OSError, BadZipFile, ET.ParseError, ValueError) as error:
        print(f"Android AAR validation failed: {error}", file=sys.stderr)
        sys.exit(1)
    print("Android AAR validation passed (ARM64, x86_64, extension, plugin and bridge).")
