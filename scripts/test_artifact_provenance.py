import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from zipfile import ZipFile

import artifact_provenance as provenance


class ProvenanceTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.addon = self.root / "addons/gdble"
        (self.addon / "android").mkdir(parents=True)
        (self.root / "src").mkdir()
        (self.root / "android-plugin").mkdir()
        for name in ["Cargo.lock", "build.rs", "src/lib.rs"]:
            (self.root / name).write_text("source\n", encoding="utf-8")
        (self.root / "Cargo.toml").write_text('[package]\nversion="0.6.0"\n', encoding="utf-8")
        for name in ["gdble.gdap", "gdble.gdextension", "permission_startup.gd"]:
            (self.addon / name).write_text("config\n", encoding="utf-8")
        self.digest = provenance.source_digest(self.root)
        for name, target in provenance.NATIVE.items():
            (self.addon / name).write_bytes(self.binary(target))
        with ZipFile(self.addon / provenance.AARS[0], "w") as archive:
            for abi, target in [("arm64-v8a", "aarch64-linux-android"), ("x86_64", "x86_64-linux-android")]:
                archive.writestr(f"jni/{abi}/libgdble.so", self.binary(target))
        (self.addon / provenance.AARS[1]).write_bytes(b"bridge")

    def binary(self, target):
        return b"native\0GDBLE_BUILD_INFO_V1:" + json.dumps({"version": "0.6.0", "source_sha256": self.digest, "source_commit": "a" * 40, "toolchain": "rustc test", "target": target, "test_harness": False}).encode() + b"\0"

    def test_content_equivalence_allows_a_separate_release_commit(self):
        provenance.generate(self.root, self.addon)
        with patch.object(provenance.subprocess, "run"):
            provenance.validate(self.root, self.addon)

    def test_changed_source_and_stale_binary_fail(self):
        (self.root / "src/lib.rs").write_text("changed", encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "digest mismatch"):
            provenance.generate(self.root, self.addon)

    def test_legacy_binary_without_marker_fails(self):
        (self.addon / next(iter(provenance.NATIVE))).write_bytes(b"old binary")
        with self.assertRaisesRegex(ValueError, "stale binary"):
            provenance.generate(self.root, self.addon)

    def test_missing_aar_fails_and_partial_record_cannot_ship(self):
        (self.addon / provenance.AARS[1]).unlink()
        with self.assertRaises(ValueError):
            provenance.generate(self.root, self.addon)
        provenance.generate(self.root, self.addon, allow_incomplete=True)
        with self.assertRaisesRegex(ValueError, "incomplete"):
            provenance.validate(self.root, self.addon)

    def test_tampering_and_unlisted_files_fail(self):
        provenance.generate(self.root, self.addon)
        (self.addon / provenance.AARS[1]).write_bytes(b"modified")
        with self.assertRaisesRegex(ValueError, "checksum"):
            provenance.validate(self.root, self.addon)

    def test_wrong_manifest_digest_fails(self):
        provenance.generate(self.root, self.addon)
        path = self.addon / "BUILD_INFO.json"
        info = json.loads(path.read_text(encoding="utf-8"))
        info["source_sha256"] = "0" * 64
        path.write_text(json.dumps(info), encoding="utf-8")
        with self.assertRaisesRegex(ValueError, "source/version"):
            provenance.validate(self.root, self.addon)

    def test_lf_normalization_matches_cross_platform_checkouts(self):
        (self.root / "src/lib.rs").write_bytes(b"source\r\n")
        self.assertEqual(provenance.source_digest(self.root), self.digest)

    def test_test_backend_must_not_ship(self):
        path = self.addon / next(iter(provenance.NATIVE))
        path.write_bytes(path.read_bytes().replace(b'"test_harness": false', b'"test_harness": true'))
        with self.assertRaisesRegex(ValueError, "test backend"):
            provenance.generate(self.root, self.addon)


if __name__ == "__main__":
    unittest.main()
