#!/usr/bin/env python3

from __future__ import annotations

import contextlib
import importlib.util
import io
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("check-version-bump.py")
SPEC = importlib.util.spec_from_file_location("check_version_bump", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
POLICY = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(POLICY)


class VersionBumpPolicyTests(unittest.TestCase):
    def test_semantic_versions_compare_numerically(self) -> None:
        self.assertTrue(POLICY.version_increased((0, 2, 0), (0, 1, 99)))
        self.assertTrue(POLICY.version_increased((1, 0, 0), (0, 99, 99)))
        self.assertFalse(POLICY.version_increased((0, 1, 1), (0, 1, 1)))
        self.assertFalse(POLICY.version_increased((0, 1, 0), (0, 1, 1)))

    def test_workspace_version_parser_rejects_non_numeric_versions(self) -> None:
        valid = b'[workspace]\n[workspace.package]\nversion = "0.1.1"\n'
        self.assertEqual(POLICY.parse_version(valid, "fixture"), ("0.1.1", (0, 1, 1)))

        invalid = b'[workspace]\n[workspace.package]\nversion = "0.1.1-dev"\n'
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            POLICY.parse_version(invalid, "fixture")


if __name__ == "__main__":
    unittest.main()
