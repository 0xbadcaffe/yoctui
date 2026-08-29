#!/usr/bin/env python3
"""Regression tests for the supported-host live design-screen gallery."""

from __future__ import annotations

import hashlib
import json
import re
import struct
import tomllib
import unittest
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
GALLERY = ROOT / "docs/design/m22/live-scenarios"
PROVENANCE = GALLERY / "manifest.toml"
README = GALLERY / "README.md"
EXPECTED_IDS = [
    "idle-dashboard",
    "active-build-tasks",
    "failed-build-errors",
    "rootfs-composition",
    "editor-application-menu",
    "terminal-sessions",
]
START = "<!-- REAL-YOCTUI-REGRESSION-SCREENS:START -->"
END = "<!-- REAL-YOCTUI-REGRESSION-SCREENS:END -->"
IMAGE = re.compile(
    r"!\[Real Yoctui regression screen: ([a-z0-9-]+)\]\(([^)]+\.png)\)"
)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def png_dimensions(path: Path) -> tuple[int, int]:
    header = path.read_bytes()[:24]
    if (
        len(header) != 24
        or header[:8] != b"\x89PNG\r\n\x1a\n"
        or header[12:16] != b"IHDR"
    ):
        raise AssertionError(f"not a PNG: {path}")
    return struct.unpack(">II", header[16:24])


class LiveDesignGalleryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        with PROVENANCE.open("rb") as manifest_file:
            cls.manifest = tomllib.load(manifest_file)
        cls.scenarios = cls.manifest["scenario"]
        source_manifest = ROOT / cls.manifest["source_manifest"]
        cls.source_manifest_path = source_manifest
        cls.source_manifest = json.loads(source_manifest.read_text(encoding="utf-8"))

    def test_gallery_has_exact_supported_capture_identity(self) -> None:
        manifest = self.manifest
        source = self.source_manifest
        self.assertEqual(manifest["schema_version"], 1)
        self.assertEqual(
            manifest["label"], "supported-live-m22-design-regression-gallery"
        )
        self.assertEqual(
            sha256(self.source_manifest_path), manifest["source_manifest_sha256"]
        )
        for field in (
            "source_commit",
            "binary_sha256",
            "host_distribution",
            "host_libc",
            "poky_revision",
            "bitbake_version",
            "machine",
            "target",
        ):
            self.assertEqual(manifest[field], source[field], field)
        self.assertEqual(manifest["logical_columns"], 160)
        self.assertEqual(manifest["logical_rows"], 50)
        self.assertEqual(manifest["pixel_width"], 1600)
        self.assertEqual(manifest["pixel_height"], 1000)

    def test_gallery_contains_exactly_six_ordered_scenarios(self) -> None:
        self.assertEqual([entry["id"] for entry in self.scenarios], EXPECTED_IDS)
        self.assertEqual(len({entry["file"] for entry in self.scenarios}), 6)
        self.assertEqual(len({entry["source"] for entry in self.scenarios}), 6)

    def test_design_pngs_match_attributed_live_rasters(self) -> None:
        live_scenarios = self.source_manifest["scenarios"]
        for entry in self.scenarios:
            scenario_id = entry["id"]
            design = ROOT / entry["file"]
            source = ROOT / entry["source"]
            self.assertTrue(design.is_file(), design)
            self.assertTrue(source.is_file(), source)
            self.assertEqual(design.read_bytes(), source.read_bytes(), scenario_id)
            self.assertEqual(sha256(design), entry["sha256"], scenario_id)
            self.assertEqual(
                entry["sha256"], live_scenarios[scenario_id]["raster_sha256"]
            )
            self.assertEqual(png_dimensions(design), (1600, 1000), scenario_id)

    def test_readme_links_each_manifest_screen_once_and_in_order(self) -> None:
        text = README.read_text(encoding="utf-8")
        self.assertEqual(text.count(START), 1)
        self.assertEqual(text.count(END), 1)
        gallery = text.split(START, 1)[1].split(END, 1)[0]
        actual = IMAGE.findall(gallery)
        expected = [
            (entry["id"], Path(entry["file"]).name) for entry in self.scenarios
        ]
        self.assertEqual(actual, expected)


if __name__ == "__main__":
    unittest.main()
