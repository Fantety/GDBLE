import importlib.util
import tempfile
import unittest
from pathlib import Path


spec = importlib.util.spec_from_file_location(
    "release_notes", Path(__file__).with_name("release-notes.py")
)
release_notes = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release_notes)


class ReleaseNotesTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        (self.root / "release-notes").mkdir()
        self.manifest = self.root / "Cargo.toml"
        self.manifest.write_text('[package]\nversion = "0.6.0"\n', encoding="utf-8")
        self.notes = self.root / "release-notes" / "v0.6.0.md"

    def test_utf8_markdown_is_selected_without_changing_its_content(self):
        content = "# GDBLE v0.6.0\n\n修复与改进。\n"
        self.notes.write_text(content, encoding="utf-8")
        selected = release_notes.release_notes_path(self.root, "v0.6.0")
        self.assertEqual(selected, self.notes)
        self.assertEqual(selected.read_text(encoding="utf-8"), content)

    def test_missing_and_empty_notes_are_rejected(self):
        with self.assertRaises(FileNotFoundError):
            release_notes.release_notes_path(self.root, "v0.6.0")
        self.notes.write_text("\n  \t\n", encoding="utf-8")
        with self.assertRaises(ValueError):
            release_notes.release_notes_path(self.root, "v0.6.0")

    def test_version_mismatch_is_rejected(self):
        with self.assertRaises(ValueError):
            release_notes.release_notes_path(self.root, "v0.7.0")

    def test_paths_and_unversioned_tags_are_rejected(self):
        for tag in ["../v0.6.0", "v0.6.0/../../secret", "vlatest", "0.6.0"]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                release_notes.release_notes_path(self.root, tag)

    def test_prerelease_uses_its_own_exact_version_file(self):
        self.manifest.write_text('[package]\nversion = "0.7.0-rc.1"\n', encoding="utf-8")
        notes = self.root / "release-notes" / "v0.7.0-rc.1.md"
        notes.write_text("候选版本说明", encoding="utf-8")
        self.assertEqual(release_notes.release_notes_path(self.root, "v0.7.0-rc.1"), notes)


if __name__ == "__main__":
    unittest.main()
