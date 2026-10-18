"""Check the archive actually downloaded by GitHub/Asset Library consumers."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest
from zipfile import ZipFile

ROOT = Path(__file__).resolve().parent.parent


class DistributionTests(unittest.TestCase):
    def test_git_archive_and_copied_addon_both_include_the_license(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            addon = root / "addons/gdble"
            addon.mkdir(parents=True)
            for name in (".gitattributes", "LICENSE", "addons/gdble/LICENSE"):
                shutil.copy2(ROOT / name, root / name)
            (addon / "gdble.gdextension").write_text("fixture\n", encoding="utf-8")
            (root / "Cargo.toml").write_text("excluded source\n", encoding="utf-8")
            subprocess.run(["git", "init", "--quiet", str(root)], check=True, capture_output=True)
            subprocess.run(["git", "add", "."], cwd=root, check=True, capture_output=True)
            tree = subprocess.check_output(["git", "write-tree"], cwd=root, text=True).strip()
            archive = root / "distribution.zip"
            subprocess.run(["git", "archive", "--format=zip", "--output", str(archive), tree], cwd=root, check=True, capture_output=True)
            with ZipFile(archive) as package:
                license_bytes = (root / "LICENSE").read_bytes().replace(b"\r\n", b"\n")
                self.assertEqual(package.read("LICENSE"), license_bytes)
                self.assertEqual(package.read("addons/gdble/LICENSE"), license_bytes)
                self.assertIn("addons/gdble/gdble.gdextension", package.namelist())
                self.assertNotIn("Cargo.toml", package.namelist())


if __name__ == "__main__":
    unittest.main()
