#!/usr/bin/env python3
"""Failure-path tests for the supported-host M22 live evidence gate."""

from __future__ import annotations

import json
import hashlib
import os
import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
VERIFIER = ROOT / "scripts/verify-live-m22-concept-evidence.py"


class LiveM22EvidenceTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temporary = tempfile.TemporaryDirectory(prefix="yoctui-m22-live-test-")
        self.evidence = Path(self.temporary.name) / "evidence"
        self.evidence.mkdir()
        # Synthetic validator inputs are generated only in an owned temporary
        # directory. They do not represent a live capture or certify a host.
        base = json.loads(
            (
                ROOT / "artifacts/release-quality/next-generation-ui/manifest.json"
            ).read_text()
        )
        base["source_commit"] = subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
        ).strip()
        self.base = Path(self.temporary.name) / "base.json"
        self.base.write_text(json.dumps(base))
        manifest = {
            **base,
            "schema": 1,
            "label": "supported-live-m22-concept-parity",
            "scenarios": {},
        }
        raster = next((ROOT / "docs/design/screenshots").rglob("*.png"))
        for scenario in (
            "idle-dashboard",
            "active-build-tasks",
            "failed-build-errors",
            "rootfs-composition",
            "editor-application-menu",
            "terminal-sessions",
        ):
            interactions = ["fixture interaction"]
            assertions = [f"fixture {scenario}"]
            files = {
                "report": (
                    f"{scenario}.json",
                    json.dumps(
                        {
                            "scenario": scenario,
                            "interactions": interactions,
                            "observed_assertions": assertions,
                        }
                    ).encode(),
                ),
                "terminal": (f"{scenario}.ansi", b"\x1b[?1049hfixture"),
                "semantic": (f"{scenario}.txt", assertions[0].encode()),
                "metadata": (f"{scenario}.meta", b"width=160\nheight=50\n"),
                "raster": (f"{scenario}.png", raster.read_bytes()),
            }
            entry = {"interactions": interactions, "observed_assertions": assertions}
            for field, (name, content) in files.items():
                (self.evidence / name).write_bytes(content)
                entry[field] = name
                entry[f"{field}_sha256"] = hashlib.sha256(content).hexdigest()
            manifest["scenarios"][scenario] = entry
        self.write_manifest(manifest)
        (self.evidence / "checksums.sha256").write_text(
            "\n".join(
                f"{hashlib.sha256(path.read_bytes()).hexdigest()}  {path.name}"
                for path in sorted(self.evidence.iterdir())
            )
            + "\n"
        )

    def tearDown(self) -> None:
        self.temporary.cleanup()

    def manifest(self) -> dict[str, object]:
        return json.loads((self.evidence / "manifest.json").read_text(encoding="utf-8"))

    def write_manifest(self, manifest: dict[str, object]) -> None:
        (self.evidence / "manifest.json").write_text(
            json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
        )

    def verify(self, expected: str, *, succeeds: bool = False) -> None:
        environment = os.environ.copy()
        environment["YOCTUI_M22_EVIDENCE"] = str(self.evidence)
        environment["YOCTUI_M22_BASE_MANIFEST"] = str(self.base)
        environment.pop("YOCTUI_LIVE_BINARY", None)
        result = subprocess.run(
            ["python3", str(VERIFIER)],
            cwd=ROOT,
            env=environment,
            capture_output=True,
            text=True,
        )
        output = result.stdout + result.stderr
        if succeeds:
            self.assertEqual(result.returncode, 0, output)
        else:
            self.assertNotEqual(result.returncode, 0, output)
        self.assertIn(expected, output)

    def test_complete_copy_passes(self) -> None:
        self.verify("6 scenarios, one binary", succeeds=True)

    def test_unsupported_host_is_rejected(self) -> None:
        manifest = self.manifest()
        manifest["host_distribution"] = "Ubuntu 26.04 LTS"
        self.write_manifest(manifest)
        self.verify("unsupported host distribution")

    def test_unattributed_interactions_are_rejected(self) -> None:
        manifest = self.manifest()
        manifest["scenarios"]["idle-dashboard"]["interactions"] = []
        self.write_manifest(manifest)
        self.verify("idle-dashboard has no explicit interactions")

    def test_stale_artifact_hash_is_rejected(self) -> None:
        manifest = self.manifest()
        manifest["scenarios"]["rootfs-composition"]["semantic_sha256"] = "0" * 64
        self.write_manifest(manifest)
        self.verify("rootfs-composition.txt does not match semantic_sha256")


if __name__ == "__main__":
    unittest.main()
