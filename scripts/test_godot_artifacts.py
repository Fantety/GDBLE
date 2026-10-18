import importlib.util
from contextlib import redirect_stdout
import io
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("godot_checks", Path(__file__).with_name("test-godot.py"))
godot_checks = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(godot_checks)


class FinalArtifactTests(unittest.TestCase):
    def setUp(self):
        resolver = patch.object(godot_checks, "resolve_godot", side_effect=lambda requested: requested)
        resolver.start()
        self.addCleanup(resolver.stop)

    def test_artifact_mode_preserves_exact_bytes_and_never_runs_cargo(self):
        with tempfile.TemporaryDirectory() as directory:
            addon = Path(directory)
            (addon / "library.dll").write_bytes(b"the final binary being reviewed")

            def execute(command, **_kwargs):
                self.assertEqual(command[0], "godot-for-test")
                project = Path(command[command.index("--path") + 1])
                self.assertEqual((project / "addons/gdble/library.dll").read_bytes(), (addon / "library.dll").read_bytes())
                return subprocess.CompletedProcess(command, 0, "GDBLE smoke test passed.\n", "")

            with patch.object(godot_checks.subprocess, "run", side_effect=execute) as runner, redirect_stdout(io.StringIO()):
                godot_checks.run("godot-for-test", "artifact", addon)
                self.assertEqual(runner.call_count, 1)
            self.assertEqual((addon / "library.dll").read_bytes(), b"the final binary being reviewed")

    def test_caught_godot_rust_panic_still_fails_verification(self):
        with tempfile.TemporaryDirectory() as directory:
            result = subprocess.CompletedProcess([], 0, "GDBLE smoke test passed.\n[panic src/file.rs:1] caught panic\n", "")
            with patch.object(godot_checks.subprocess, "run", return_value=result), redirect_stdout(io.StringIO()):
                with self.assertRaises(SystemExit):
                    godot_checks.run("godot", "artifact", Path(directory))


class GodotExecutableTests(unittest.TestCase):
    def test_setup_action_alias_without_extension_and_with_spaces_is_resolved(self):
        with tempfile.TemporaryDirectory(prefix="godot action ") as directory:
            alias = Path(directory) / "godot"
            alias.write_bytes(b"fixture")
            with patch.dict(os.environ, {"GODOT": str(alias)}, clear=True), patch.object(godot_checks.shutil, "which", return_value=None):
                self.assertEqual(godot_checks.resolve_godot(), str(alias.resolve()))

    def test_explicit_executable_takes_precedence_over_action_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            executable = Path(directory) / "chosen.exe"
            executable.write_bytes(b"fixture")
            with patch.dict(os.environ, {"GODOT": "wrong-version"}, clear=True):
                self.assertEqual(godot_checks.resolve_godot(str(executable)), str(executable.resolve()))

    def test_godot4_and_path_fallbacks(self):
        with tempfile.TemporaryDirectory() as directory:
            executable = Path(directory) / "godot.exe"
            executable.write_bytes(b"fixture")
            with patch.dict(os.environ, {"GODOT4": str(executable)}, clear=True):
                self.assertEqual(godot_checks.resolve_godot(), str(executable.resolve()))
            with patch.dict(os.environ, {}, clear=True), patch.object(godot_checks.shutil, "which", return_value=str(executable)) as locate:
                self.assertEqual(godot_checks.resolve_godot(), str(executable.resolve()))
                locate.assert_called_once_with("godot")

    def test_invalid_explicit_path_fails_before_any_build(self):
        with patch.dict(os.environ, {"GODOT": "another-engine"}, clear=True), patch.object(godot_checks.shutil, "which", return_value=None), patch.object(godot_checks.subprocess, "run") as runner:
            with self.assertRaisesRegex(SystemExit, "Godot executable not found"):
                godot_checks.run("missing-godot", "source", Path("unused-addon"))
            runner.assert_not_called()


if __name__ == "__main__":
    unittest.main()
