"""Record and validate the exact addon. Validation never invokes a compiler."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tomllib
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parent.parent
NATIVE = {
    "libgdble.windows.x86_64.dll": "x86_64-pc-windows-msvc",
    "libgdble.linux.x86_64.so": "x86_64-unknown-linux-gnu",
    "libgdble.macos.x86_64.dylib": "x86_64-apple-darwin",
    "libgdble.macos.arm64.dylib": "aarch64-apple-darwin",
}
AARS = ("android/gdble-release.aar", "android/btleplug-release.aar")
REQUIRED = {*NATIVE, *AARS, "gdble.gdextension", "gdble.gdap", "permission_startup.gd", "LICENSE"}


def check_license(root: Path, addon: Path) -> None:
    expected = (root / "LICENSE").read_text(encoding="utf-8")
    actual = (addon / "LICENSE").read_text(encoding="utf-8")
    if not expected.strip() or actual != expected:
        raise ValueError("addon LICENSE must match the repository LICENSE")


def source_digest(root: Path) -> str:
    """Identical length-prefixed UTF-8/LF content inventory to build.rs."""
    paths = [root / name for name in ("Cargo.toml", "Cargo.lock", "build.rs", "addons/gdble/gdble.gdextension", "addons/gdble/gdble.gdap", "addons/gdble/permission_startup.gd")]
    for directory in ("src", "android-plugin"):
        paths.extend(p for p in (root / directory).rglob("*")
                     if p.is_file() and p.name != "local.properties" and p.suffix in {".rs", ".kt", ".kts", ".xml", ".pro", ".properties"}
                     and not {"build", ".gradle"}.intersection(p.relative_to(root / directory).parts))
    digest = hashlib.sha256()
    for path in sorted(paths, key=lambda p: p.relative_to(root).as_posix()):
        name = path.relative_to(root).as_posix().encode("utf-8")
        content = path.read_bytes().decode("utf-8").replace("\r\n", "\n").encode("utf-8")
        for value in (name, content):
            digest.update(len(value).to_bytes(8, "big"))
            digest.update(value)
    return digest.hexdigest()


def embedded_info(data: bytes) -> dict:
    markers = re.findall(rb"GDBLE_BUILD_INFO_V1:(\{[^\x00]+\})\x00", data)
    if len(markers) != 1:
        raise ValueError("missing or ambiguous embedded build information (stale binary)")
    return json.loads(markers[0])


def artifact_info(addon: Path, name: str) -> dict:
    if name in NATIVE:
        return {NATIVE[name]: embedded_info((addon / name).read_bytes())}
    if name == AARS[0]:
        with ZipFile(addon / name) as archive:
            return {target: embedded_info(archive.read(f"jni/{abi}/libgdble.so")) for abi, target in (
                ("arm64-v8a", "aarch64-linux-android"), ("x86_64", "x86_64-linux-android"))}
    return {}


def check_info(info: dict, digest: str, version: str) -> None:
    for target, build in info.items():
        if build.get("source_sha256") != digest:
            raise ValueError(f"{target}: source digest mismatch")
        if build.get("version") != version or build.get("target") != target:
            raise ValueError(f"{target}: version/target mismatch")
        if build.get("test_harness") is not False:
            raise ValueError(f"{target}: test backend must not ship")
        if not build.get("toolchain") or not re.fullmatch(r"[0-9a-f]{40}", build.get("source_commit", "")):
            raise ValueError(f"{target}: missing toolchain/source commit")


def files(addon: Path) -> dict[str, str]:
    return {p.relative_to(addon).as_posix(): hashlib.sha256(p.read_bytes()).hexdigest()
            for p in sorted(addon.rglob("*")) if p.is_file() and p.name != "SHA256SUMS"}


def generate(root: Path, addon: Path, allow_incomplete: bool = False) -> dict:
    digest = source_digest(root)
    version = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    artifacts, failures = {}, []
    for name in sorted(REQUIRED):
        try:
            data = (addon / name).read_bytes()
            if not data:
                raise ValueError("empty artifact")
            if name == "LICENSE":
                check_license(root, addon)
            info = artifact_info(addon, name)
            check_info(info, digest, version)
            artifacts[name] = {"sha256": hashlib.sha256(data).hexdigest(), "builds": info}
        except (OSError, ValueError, KeyError) as error:
            failures.append(f"{name}: {error}")
    if failures and not allow_incomplete:
        raise ValueError("\n".join(failures))
    report = {"schema": 1, "version": version, "source_sha256": digest,
              "release_ready": not failures, "incomplete": failures, "artifacts": artifacts}
    # Match .gitattributes on every host: Git must not change these bytes after
    # BUILD_INFO.json has been included in the checksum inventory.
    (addon / "BUILD_INFO.json").write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8", newline="\n")
    (addon / "SHA256SUMS").write_text("".join(f"{hash_value}  {name}\n" for name, hash_value in files(addon).items()), encoding="utf-8", newline="\n")
    return report


def validate(root: Path, addon: Path) -> None:
    report = json.loads((addon / "BUILD_INFO.json").read_text(encoding="utf-8"))
    if report.get("schema") != 1 or not report.get("release_ready") or report.get("incomplete"):
        failures = "\n".join(str(item) for item in report.get("incomplete", []))
        raise ValueError("addon is incomplete; run Prepare release and merge the rebuilt addon before tagging.\n" + failures)
    check_license(root, addon)
    digest = source_digest(root)
    version = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["package"]["version"]
    if report["source_sha256"] != digest or report["version"] != version:
        raise ValueError("build manifest source/version mismatch")
    expected = {}
    for line in (addon / "SHA256SUMS").read_text(encoding="utf-8").splitlines():
        value, name = line.split("  ", 1)
        if name in expected or not re.fullmatch("[0-9a-f]{64}", value):
            raise ValueError("invalid checksum manifest")
        expected[name] = value
    if files(addon) != expected:
        raise ValueError("checksum mismatch, missing file, or unlisted addon file")
    if set(report["artifacts"]) != REQUIRED:
        raise ValueError("required artifact inventory mismatch")
    for name in sorted(REQUIRED):
        entry = report["artifacts"][name]
        if entry["sha256"] != expected[name]:
            raise ValueError(f"{name}: artifact hash mismatch")
        info = artifact_info(addon, name)
        if info != entry["builds"]:
            raise ValueError(f"{name}: embedded metadata mismatch")
        check_info(info, digest, version)
    # Share the structural checks used by Android builds.
    subprocess.run([sys.executable, str(ROOT / "scripts/validate-android-aar.py"), str(addon / "android")], check=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["generate", "validate", "digest"])
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--addon", type=Path)
    parser.add_argument("--allow-incomplete", action="store_true", help="record local progress; tag validation will reject this manifest")
    args = parser.parse_args()
    try:
        if args.mode == "digest":
            print(source_digest(args.root))
        elif args.mode == "generate":
            report = generate(args.root, args.addon or args.root / "addons/gdble", args.allow_incomplete)
            print(json.dumps({"release_ready": report["release_ready"], "incomplete": report["incomplete"]}, indent=2))
        else:
            validate(args.root, args.addon or args.root / "addons/gdble")
            print("Final addon provenance and hashes passed.")
    except (OSError, ValueError, KeyError, subprocess.CalledProcessError) as error:
        raise SystemExit(f"Artifact validation failed: {error}") from error
