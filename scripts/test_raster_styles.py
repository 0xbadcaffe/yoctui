"""Native terminal style projection regressions for README screenshots."""
import runpy
import tempfile
import unittest
from pathlib import Path

RASTER = runpy.run_path(str(Path(__file__).with_name("render-m22-concept-screenshots.py")))


class RasterStyles(unittest.TestCase):
    def parse(self, modifiers, foreground="Red", background="Blue"):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "test.cells"
            source.write_text(
                "YOCTUI_CELL_GOLDEN_V1 160 50\nSYMBOLS\n"
                + ("S|" + "1: " * 160 + "\n") * 50
                + f"STYLES\nT|8000|fg={foreground};bg={background};ul=Reset;mod={modifiers}\n"
            )
            return RASTER["parse_cell_golden"](source)[1][0]

    def test_normal_bold_underline_and_reverse(self):
        normal = self.parse("NONE")
        self.assertFalse(normal.bold)
        self.assertFalse(normal.underline)
        native = self.parse("BOLD | UNDERLINED | REVERSED")
        self.assertTrue(native.bold)
        self.assertTrue(native.underline)
        self.assertEqual(native.foreground, normal.background)
        self.assertEqual(native.background, normal.foreground)

    def test_dim_blends_foreground_without_dimming_background(self):
        normal = self.parse("NONE")
        dim = self.parse("DIM")
        self.assertEqual(dim.foreground, (127, 0, 127))
        self.assertEqual(dim.background, normal.background)
        self.assertFalse(dim.bold)
        self.assertFalse(dim.underline)

    def test_dim_preserves_bold_underline_and_effective_reversed_background(self):
        dim = self.parse(
            "BOLD | DIM | UNDERLINED | REVERSED",
            "Rgb(200, 100, 20)", "Rgb(10, 40, 90)",
        )
        self.assertEqual(dim.foreground, (105, 70, 55))
        self.assertEqual(dim.background, (200, 100, 20))
        self.assertTrue(dim.bold)
        self.assertTrue(dim.underline)

    def test_dim_uses_effective_reset_colors(self):
        dim = self.parse("DIM", "Reset", "Reset")
        fg, bg = RASTER["DEFAULT_FOREGROUND"], RASTER["DEFAULT_BACKGROUND"]
        self.assertEqual(
            dim.foreground,
            tuple((front + back) // 2 for front, back in zip(fg, bg)),
        )
        self.assertEqual(dim.background, bg)

    def test_unsupported_style_is_not_silently_discarded(self):
        for modifiers in ("SLOW_BLINK", "DIM | SLOW_BLINK", "UNKNOWN", "ITALIC"):
            with self.subTest(modifiers=modifiers), self.assertRaises(SystemExit):
                self.parse(modifiers)


if __name__ == "__main__":
    unittest.main()
