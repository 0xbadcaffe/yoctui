#!/usr/bin/env python3
"""Regression checks for Unicode terminal-cell raster graphics."""
import importlib.util
from pathlib import Path
import sys
import unittest

import cairo

spec = importlib.util.spec_from_file_location(
    "concept_raster", Path(__file__).with_name("render-m22-concept-screenshots.py")
)
assert spec and spec.loader
raster = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = raster
spec.loader.exec_module(raster)


class CellGraphicsTests(unittest.TestCase):
    def draw(self, symbol):
        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, 10, 20)
        context = cairo.Context(surface)
        context.set_source_rgba(1, 1, 1, 1)
        handled = raster.draw_cell_graphic(context, symbol, 0, 0)
        surface.flush()
        return handled, bytes(surface.get_data())

    def test_all_braille_patterns_have_distinct_pixels(self):
        patterns = [self.draw(chr(0x2800 + mask)) for mask in range(256)]
        self.assertTrue(all(handled for handled, _ in patterns))
        self.assertEqual(len({pixels for _, pixels in patterns}), 256)
        self.assertFalse(any(patterns[0][1]))
        for mask in range(1, 256):
            self.assertTrue(any(patterns[mask][1]))

    def test_borders_reach_cell_edges(self):
        for symbol, positions in [("─", [(0, 10), (9, 10)]), ("│", [(5, 0), (5, 19)])]:
            handled, pixels = self.draw(symbol)
            self.assertTrue(handled)
            for x, y in positions:
                self.assertTrue(any(pixels[(y * 10 + x) * 4:(y * 10 + x + 1) * 4]))

    def test_text_uses_pinned_font_and_blocks_fill_exact_cells(self):
        self.assertFalse(self.draw("A")[0])
        self.assertFalse(self.draw("ab")[0])
        handled, pixels = self.draw("█")
        self.assertTrue(handled)
        self.assertTrue(all(pixels))
        _, lower = self.draw("▄")
        self.assertFalse(any(lower[:10 * 10 * 4]))
        self.assertTrue(all(lower[10 * 10 * 4:]))


if __name__ == "__main__":
    unittest.main()
