#!/usr/bin/env python3
"""Failure-path tests for the M21 concept-screen verifier."""

from __future__ import annotations

import contextlib
import importlib.util
import io
import re
import shutil
import sys
import tempfile
import unittest
from pathlib import Path


sys.dont_write_bytecode = True
SOURCE_ROOT = Path(__file__).resolve().parents[1]
VERIFIER = SOURCE_ROOT / "scripts" / "verify-m21-concept-screens.py"


def load_verifier():
    spec = importlib.util.spec_from_file_location("m21_concept_screen_verifier", VERIFIER)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load concept-screen verifier")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ConceptScreenVerifierTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="yoctui-concept-verifier-")
        self.root = Path(self.temporary.name)
        for relative in [
            "docs/design/m21/concepts/manifest.toml",
            "docs/task-registry.toml",
        ]:
            destination = self.root / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(SOURCE_ROOT / relative, destination)
        golden_source = SOURCE_ROOT / "crates/yoctui-ui/tests/golden"
        golden_destination = self.root / "crates/yoctui-ui/tests/golden"
        golden_destination.mkdir(parents=True)
        for source in golden_source.glob("concept-*.*"):
            shutil.copy2(source, golden_destination / source.name)

        self.verifier = load_verifier()
        self.verifier.ROOT = self.root
        self.verifier.MANIFEST = self.root / "docs/design/m21/concepts/manifest.toml"
        self.verifier.REGISTRY = self.root / "docs/task-registry.toml"

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def run_verifier(self) -> str:
        output = io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            self.verifier.main()
        return output.getvalue()

    def assert_rejected(self, message: str) -> None:
        output = io.StringIO()
        with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
            with self.assertRaises(SystemExit) as raised:
                self.verifier.main()
        self.assertEqual(raised.exception.code, 1)
        self.assertIn(message, output.getvalue())

    def test_accepts_reviewed_pack(self) -> None:
        self.assertIn("6 production-renderer scenes", self.run_verifier())

    def test_rejects_missing_semantic_anchor(self) -> None:
        path = self.verifier.MANIFEST
        text = path.read_text(encoding="utf-8").replace(
            '"Current Build · Idle"', '"anchor that cannot exist"', 1
        )
        path.write_text(text, encoding="utf-8")
        self.assert_rejected("semantic capture is missing anchor")

    def test_rejects_corrupt_cell_dimensions(self) -> None:
        path = (
            self.root
            / "crates/yoctui-ui/tests/golden/concept-idle-dashboard-160x50.cells"
        )
        text = path.read_text(encoding="utf-8").replace(
            "YOCTUI_CELL_GOLDEN_V1 160 50",
            "YOCTUI_CELL_GOLDEN_V1 159 50",
            1,
        )
        path.write_text(text, encoding="utf-8")
        self.assert_rejected("invalid cell-golden header")

    def test_rejects_missing_real_renderer_capture(self) -> None:
        path = (
            self.root
            / "crates/yoctui-ui/tests/golden/concept-terminal-sessions-160x50.txt"
        )
        path.unlink()
        self.assert_rejected("missing crates/yoctui-ui/tests/golden")

    def test_rejects_completed_task_with_open_gap(self) -> None:
        path = self.verifier.MANIFEST
        text = path.read_text(encoding="utf-8")
        text, replacements = re.subn(
            r'(implementation_tasks = \[[^\n]*"UX-DASHBOARD-001"[^\n]*\]\n'
            r'open_gaps = \[\n)',
            r'\1  { task = "UX-DASHBOARD-001", description = "test gap" },\n',
            text,
            count=1,
        )
        self.assertEqual(replacements, 1)
        path.write_text(text, encoding="utf-8")
        self.assert_rejected("completed task UX-DASHBOARD-001 still owns an open gap")


if __name__ == "__main__":
    unittest.main()
