"""Native terminal style projection regressions for README screenshots."""
import runpy
import tempfile
import unittest
from pathlib import Path

RASTER = runpy.run_path(str(Path(__file__).with_name("render-m22-concept-screenshots.py")))


class RasterStyles(unittest.TestCase):
    def parse(self, modifiers):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "test.cells"
            source.write_text(
                "YOCTUI_CELL_GOLDEN_V1 160 50\nSYMBOLS\n"
                + ("S|" + "1: " * 160 + "\n") * 50
                + f"STYLES\nT|8000|fg=Red;bg=Blue;ul=Reset;mod={modifiers}\n"
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

    def test_unsupported_style_is_not_silently_discarded(self):
        with self.assertRaises(SystemExit):
            self.parse("SLOW_BLINK")


if __name__ == "__main__":
    unittest.main()
