#!/usr/bin/env python3
"""Regression tests for styled real-PTY screen composition."""

from __future__ import annotations

import importlib.util
import sys
import unittest
from pathlib import Path


MODULE_PATH = Path(__file__).with_name("terminal_capture.py")
SPEC = importlib.util.spec_from_file_location("terminal_capture", MODULE_PATH)
assert SPEC and SPEC.loader
MODULE = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)
Screen = MODULE.Screen


class TerminalCaptureTests(unittest.TestCase):
    def test_cursor_addressing_and_truecolor_survive_composition(self) -> None:
        screen = Screen(4, 2)
        screen.feed(b"\x1b[2J\x1b[2;2H\x1b[38;2;42;178;218;48;2;4;12;17;1mOK")

        self.assertEqual(screen.text(), "\n OK\n")
        first = screen.cells[1][1]
        self.assertEqual(first.symbol, "O")
        self.assertEqual(first.style.foreground, "Rgb(42, 178, 218)")
        self.assertEqual(first.style.background, "Rgb(4, 12, 17)")
        self.assertTrue(first.style.bold)

    def test_cell_golden_has_exact_bounded_symbol_and_style_coverage(self) -> None:
        screen = Screen(3, 1)
        screen.feed(b"A\x1b[38;2;139;211;0mB\x1b[0mC")
        golden = screen.cell_golden().splitlines()

        self.assertEqual(golden[:2], ["YOCTUI_CELL_GOLDEN_V1 3 1", "SYMBOLS"])
        self.assertEqual(golden[2], "S|1:A1:B1:C")
        self.assertEqual(golden[3], "STYLES")
        self.assertEqual(sum(int(line.split("|")[1]) for line in golden[4:]), 3)
        self.assertTrue(any("fg=Rgb(139, 211, 0)" in line for line in golden[4:]))

    def test_erase_replaces_stale_symbol_and_style(self) -> None:
        screen = Screen(4, 1)
        screen.feed(b"\x1b[38;2;244;67;54mFAIL\x1b[1;2H\x1b[0m\x1b[K")

        self.assertEqual(screen.text(), "F\n")
        self.assertEqual(screen.cells[0][1].symbol, " ")
        self.assertEqual(screen.cells[0][1].style.foreground, "Reset")


if __name__ == "__main__":
    unittest.main()
