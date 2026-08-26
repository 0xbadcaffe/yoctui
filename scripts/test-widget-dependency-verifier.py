#!/usr/bin/env python3
"""Failure-path tests for the widget dependency admission verifier."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path

SCRIPT = Path(__file__).with_name("third_party_compliance.py")
sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location("third_party_compliance", SCRIPT)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class CandidateVerifierTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory(prefix="yoctui-widget-verifier-")
        self.root = Path(self.temp.name)
        self.audit = tomllib.loads(MODULE.AUDIT_PATH.read_text(encoding="utf-8"))
        self.sbom = json.loads(MODULE.CANDIDATE_SBOM_PATH.read_text(encoding="utf-8"))

    def tearDown(self) -> None:
        self.temp.cleanup()

    def write(self) -> tuple[Path, Path]:
        audit_path = self.root / "audit.toml"
        audit_text = MODULE.AUDIT_PATH.read_text(encoding="utf-8")
        for candidate in self.audit["candidate"]:
            original = next(item for item in tomllib.loads(audit_text)["candidate"] if item["name"] == candidate["name"])
            for key, value in candidate.items():
                if value == original.get(key):
                    continue
                old = f'{key} = "{original[key]}"' if isinstance(original[key], str) else f"{key} = {str(original[key]).lower()}"
                new = f'{key} = "{value}"' if isinstance(value, str) else f"{key} = {str(value).lower()}"
                audit_text = audit_text.replace(old, new, 1)
        audit_path.write_text(audit_text, encoding="utf-8")
        sbom_path = self.root / "sbom.json"
        sbom_path.write_text(json.dumps(self.sbom), encoding="utf-8")
        return audit_path, sbom_path

    def assert_rejected(self, message: str) -> None:
        audit_path, sbom_path = self.write()
        with self.assertRaisesRegex(ValueError, message):
            MODULE.validate_candidate_audit(audit_path, sbom_path)

    def test_valid_evidence(self) -> None:
        audit_path, sbom_path = self.write()
        MODULE.validate_candidate_audit(audit_path, sbom_path)

    def test_rejects_invalid_checksum(self) -> None:
        self.audit["candidate"][0]["checksum"] = "bad"
        self.assert_rejected("invalid crate checksum")

    def test_rejects_default_features(self) -> None:
        self.audit["candidate"][0]["default_features"] = True
        self.assert_rejected("default features must be disabled")

    def test_rejects_missing_candidate_component(self) -> None:
        removed = next(
            component
            for component in self.sbom["components"]
            if component["name"] == "ratatui-image"
        )
        self.sbom["components"].remove(removed)
        self.sbom["dependencies"] = [item for item in self.sbom["dependencies"] if item["ref"] != removed["bom-ref"]]
        for item in self.sbom["dependencies"]:
            item["dependsOn"] = [ref for ref in item["dependsOn"] if ref != removed["bom-ref"]]
        self.assert_rejected("candidate SBOM roots do not match")

    def test_rejects_dangling_dependency(self) -> None:
        self.sbom["dependencies"][0]["dependsOn"].append("pkg:cargo/missing@1.0.0")
        self.assert_rejected("dangling dependencies")


if __name__ == "__main__":
    unittest.main()
