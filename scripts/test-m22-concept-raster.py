#!/usr/bin/env python3
"""Regression tests for deterministic production-cell raster evidence."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import tomllib
import unittest
from pathlib import Path


sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/render-m22-concept-screenshots.py"
SPEC = importlib.util.spec_from_file_location("m22_raster", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
RASTER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = RASTER
SPEC.loader.exec_module(RASTER)


class ConceptRasterTests(unittest.TestCase):
    def test_checked_artifacts_are_exactly_reproducible(self) -> None:
        subprocess.run([str(SCRIPT), "--check"], cwd=ROOT, check=True)

    def test_every_artifact_has_exact_dimensions_and_source_hash(self) -> None:
        with RASTER.PROVENANCE.open("rb") as manifest_file:
            manifest = tomllib.load(manifest_file)
        artifacts = manifest["artifact"]
        self.assertEqual(len(artifacts), 6)
        self.assertEqual(manifest["pixel_width"], 1600)
        self.assertEqual(manifest["pixel_height"], 1000)
        for artifact in artifacts:
            source = ROOT / artifact["source"]
            image = ROOT / artifact["file"]
            self.assertEqual(RASTER.sha256(source), artifact["source_sha256"])
            self.assertEqual(RASTER.sha256(image), artifact["sha256"])
            self.assertEqual(RASTER.png_dimensions(image), (1600, 1000))

    def test_single_scene_render_is_byte_deterministic(self) -> None:
        _, source = RASTER.scenarios()[0]
        with tempfile.TemporaryDirectory() as temporary:
            first = Path(temporary) / "first.png"
            second = Path(temporary) / "second.png"
            RASTER.render_cell_golden(source, first)
            RASTER.render_cell_golden(source, second)
            self.assertEqual(first.read_bytes(), second.read_bytes())

    def test_symbol_parser_rejects_truncated_utf8_cell(self) -> None:
        with self.assertRaises(SystemExit):
            RASTER.parse_symbol_row("S|3:─"[:-1])

    def test_live_terminal_indexed_colors_have_exact_rgb_projection(self) -> None:
        default = (1, 2, 3)
        self.assertEqual(RASTER.parse_color("DarkGray", default), (128, 128, 128))
        self.assertEqual(RASTER.parse_color("Indexed(16)", default), (0, 0, 0))
        self.assertEqual(RASTER.parse_color("Indexed(196)", default), (255, 0, 0))
        self.assertEqual(RASTER.parse_color("Indexed(255)", default), (238, 238, 238))

    def test_renderer_and_font_identity_are_pinned(self) -> None:
        RASTER.require_renderer()
        self.assertEqual(RASTER.sha256(RASTER.REGULAR_FONT), RASTER.REGULAR_FONT_SHA256)
        self.assertEqual(RASTER.sha256(RASTER.BOLD_FONT), RASTER.BOLD_FONT_SHA256)


if __name__ == "__main__":
    unittest.main()
