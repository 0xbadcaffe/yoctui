from copy import deepcopy
from pathlib import Path
import hashlib
import json
import unittest
from unittest.mock import patch

from scripts.verify_ipc_historical_sources import verify_sources


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "artifacts/performance/ipc-gate/manifest.json"


class HistoricalIpcSourceTests(unittest.TestCase):
    def setUp(self):
        self.manifest = json.loads(MANIFEST.read_text())

    def test_recorded_revision_plus_patch_passes_without_changing_checkout(self):
        paths = [MANIFEST] + [ROOT / name for name in self.manifest["sources"]]
        before = {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}
        verify_sources(self.manifest, ROOT)
        after = {str(path): hashlib.sha256(path.read_bytes()).hexdigest() for path in paths}
        self.assertEqual(before, after)

    def test_changed_source_digest_fails(self):
        altered = deepcopy(self.manifest)
        altered["sources"]["scripts/event-flood-harness.py"] = "0" * 64
        with self.assertRaisesRegex(ValueError, "source digest mismatch"):
            verify_sources(altered, ROOT)

    def test_tampered_recorded_patch_fails(self):
        patch_path = ROOT / "artifacts/performance/ipc-gate/v91-readiness-source.patch"
        original_read = Path.read_bytes
        with patch.object(Path, "read_bytes", autospec=True) as reader:
            reader.side_effect = lambda path: b"tampered" if path == patch_path else original_read(path)
            with self.assertRaisesRegex(ValueError, "patch digest mismatch"):
                verify_sources(self.manifest, ROOT)

    def test_invalid_revision_or_unsafe_path_fails(self):
        for field, value in (("source_base_revision", "HEAD"), ("sources", {"../outside": "0" * 64, "capture.patch": "0" * 64})):
            altered = deepcopy(self.manifest)
            altered[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                verify_sources(altered, ROOT)

    def test_missing_recorded_patch_fails(self):
        altered = deepcopy(self.manifest)
        altered["sources"] = {name: digest for name, digest in altered["sources"].items() if not name.endswith(".patch")}
        with self.assertRaisesRegex(ValueError, "exactly one"):
            verify_sources(altered, ROOT)


if __name__ == "__main__":
    unittest.main()
