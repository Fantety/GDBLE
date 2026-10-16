import importlib.util
from contextlib import redirect_stdout
import io
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

SPEC = importlib.util.spec_from_file_location("godot_checks", Path(__file__).with_name("test-godot.py"))
godot_checks = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(godot_checks)


class FinalArtifactTests(unittest.TestCase):
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


if __name__ == "__main__":
    unittest.main()
