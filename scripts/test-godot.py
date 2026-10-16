"""Run isolated BLE, permission, and debugger tests, or verify final addon bytes."""
import argparse
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def run(godot: str, mode: str, addon: Path, visual_dir: Path | None = None) -> None:
    root = ROOT / "target"
    root.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="godot-" + mode + "-", dir=root) as temporary:
        project = Path(temporary)
        staged = project / "addons/gdble"
        shutil.copytree(addon, staged)
        (project / "project.godot").write_text('config_version=5\n[application]\nconfig/name="GDBLE verification"\n[rendering]\nrenderer/rendering_method="gl_compatibility"\n', encoding="utf-8")
        if mode in ("source", "harness", "debugger"):
            build = ROOT / "target" / ("godot-harness-build" if mode == "harness" else "godot-source-build")
            command = ["cargo", "build", "--locked", "--target-dir", str(build)]
            if mode == "harness":
                command += ["--features", "test-harness"]
            subprocess.run(command, cwd=ROOT, check=True)
            system = platform.system()
            native, packaged = {
                "Windows": ("gdble.dll", "libgdble.windows.x86_64.dll"),
                "Linux": ("libgdble.so", "libgdble.linux.x86_64.so"),
                "Darwin": ("libgdble.dylib", "libgdble.macos." + ("arm64" if platform.machine() == "arm64" else "x86_64") + ".dylib"),
            }[system]
            shutil.copy2(build / "debug" / native, staged / packaged)
        script = {"harness": "reentry_test.gd", "permissions": "permission_test.gd", "debugger": "debugger/tests/debugger_test.gd"}.get(mode, "smoke_test.gd")
        if mode == "debugger":
            shutil.copytree(ROOT / "demo/debugger", project / "debugger", ignore=shutil.ignore_patterns("*.uid"))
            shutil.copy2(ROOT / "demo/node_2d.tscn", project / "node_2d.tscn")
            shutil.copy2(ROOT / "demo/project.godot", project / "project.godot")
        else:
            shutil.copy2(ROOT / "demo" / script, project / script)
        command = [godot, "--path", str(project), "--script", script]
        if visual_dir is not None:
            command += ["--", "--visual-dir", str(visual_dir.resolve())]
        else:
            command.insert(1, "--headless")
        try:
            result = subprocess.run(command, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=45, env={**os.environ, "GODOT_SILENCE_ROOT_WARNING": "1"})
        except subprocess.TimeoutExpired as error:
            for output in (error.stdout, error.stderr):
                if output:
                    print(output.decode("utf-8", errors="replace") if isinstance(output, bytes) else output, end="")
            raise SystemExit("Godot verification timed out") from error
        print(result.stdout, end="")
        print(result.stderr, end="")
        marker = "GDBLE " + {"harness": "reentry", "permissions": "permission", "debugger": "debugger"}.get(mode, "smoke") + " test passed."
        diagnostics = result.stdout + result.stderr
        if result.returncode or "ERROR" in diagnostics or "panic" in diagnostics.lower() or marker not in result.stdout:
            raise SystemExit("Godot verification failed")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--godot", default="godot")
    parser.add_argument("--mode", choices=["source", "harness", "artifact", "permissions", "debugger"], required=True)
    parser.add_argument("--addon", type=Path, default=ROOT / "addons/gdble")
    parser.add_argument("--visual-dir", type=Path, help="Render debugger layout screenshots using a visible window")
    args = parser.parse_args()
    if args.visual_dir and args.mode != "debugger":
        parser.error("--visual-dir requires --mode debugger")
    run(args.godot, args.mode, args.addon, args.visual_dir)
