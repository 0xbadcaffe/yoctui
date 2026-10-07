from copy import deepcopy
from pathlib import Path
import hashlib
import subprocess
import tempfile
import unittest
from unittest.mock import patch

from scripts.verify_ipc_historical_sources import verify_sources


class HistoricalIpcSourceTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="yoctui-ipc-fixture-")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.source = "scripts/event-flood-harness.py"
        self.patch = "capture.patch"
        source = self.root / self.source
        source.parent.mkdir()
        source.write_text("# original fixture\n")

        def git(*args):
            return subprocess.check_output(["git", *args], cwd=self.root)

        git("init", "-q")
        git("add", self.source)
        git(
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "-qm",
            "Fixture base",
        )
        revision = git("rev-parse", "HEAD").decode().strip()
        source.write_text("# patched fixture\n")
        patch_path = self.root / self.patch
        patch_path.write_bytes(git("diff", "--", self.source))
        self.manifest = {
            "source_base_revision": revision,
            "sources": {
                name: hashlib.sha256((self.root / name).read_bytes()).hexdigest()
                for name in (self.source, self.patch)
            },
        }

    def test_recorded_revision_plus_patch_passes_without_changing_checkout(self):
        paths = [self.root / name for name in self.manifest["sources"]]
        before = {
            str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths
        }
        verify_sources(self.manifest, self.root)
        after = {
            str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths
        }
        self.assertEqual(before, after)

    def test_changed_source_digest_fails(self):
        altered = deepcopy(self.manifest)
        altered["sources"]["scripts/event-flood-harness.py"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "source digest mismatch"):
            verify_sources(altered, self.root)

    def test_tampered_recorded_patch_fails(self):
        patch_path = self.root / self.patch
        original_read = Path.read_bytes
        with patch.object(Path, "read_bytes", autospec=True) as reader:
            reader.side_effect = lambda path: (
                b"tampered" if path == patch_path else original_read(path)
            )
            with self.assertRaisesRegex(ValueError, "patch digest mismatch"):
                verify_sources(self.manifest, self.root)

    def test_invalid_revision_or_unsafe_path_fails(self):
        for field, value in (
            ("source_base_revision", "HEAD"),
            ("sources", {"../outside": "0" * 64, "capture.patch": "0" * 64}),
        ):
            altered = deepcopy(self.manifest)
            altered[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                verify_sources(altered, self.root)

    def test_missing_recorded_patch_fails(self):
        altered = deepcopy(self.manifest)
        altered["sources"] = {
            name: digest
            for name, digest in altered["sources"].items()
            if not name.endswith(".patch")
        }
        with self.assertRaisesRegex(ValueError, "exactly one"):
            verify_sources(altered, self.root)


if __name__ == "__main__":
    unittest.main()
