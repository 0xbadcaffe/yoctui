"""Small bounded terminal compositor used by Yoctui's live capture gates."""

from __future__ import annotations

import codecs
import re
import unicodedata
from dataclasses import dataclass


CSI = re.compile(r"\x1b\[([0-9;?]*)([ -/]*)?([@-~])")


@dataclass(frozen=True)
class Style:
    foreground: str = "Reset"
    background: str = "Reset"
    bold: bool = False


@dataclass
class Cell:
    symbol: str = " "
    style: Style = Style()


class Screen:
    """Compose cursor-addressed UTF-8/SGR output into symbols and styles."""

    def __init__(self, width: int, height: int) -> None:
        self.width = width
        self.height = height
        self.cells = [[Cell() for _ in range(width)] for _ in range(height)]
        self.x = 0
        self.y = 0
        self.saved = (0, 0)
        self.style = Style()
        self.pending = ""
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")

    def clear(self) -> None:
        self.cells = [[Cell(style=self.style) for _ in range(self.width)] for _ in range(self.height)]
        self.x = 0
        self.y = 0

    def text(self) -> str:
        return "\n".join("".join(cell.symbol for cell in row).rstrip() for row in self.cells).rstrip() + "\n"

    def cell_golden(self) -> str:
        lines = [f"YOCTUI_CELL_GOLDEN_V1 {self.width} {self.height}", "SYMBOLS"]
        for row in self.cells:
            encoded = []
            for cell in row:
                raw = cell.symbol.encode("utf-8")
                encoded.append(f"{len(raw)}:{cell.symbol}")
            lines.append("S|" + "".join(encoded))
        lines.append("STYLES")
        runs: list[tuple[int, Style]] = []
        for cell in (cell for row in self.cells for cell in row):
            if runs and runs[-1][1] == cell.style:
                runs[-1] = (runs[-1][0] + 1, cell.style)
            else:
                runs.append((1, cell.style))
        lines.extend(
            f"T|{count}|fg={style.foreground};bg={style.background};ul=Reset;mod={'BOLD' if style.bold else 'NONE'}"
            for count, style in runs
        )
        return "\n".join(lines) + "\n"

    def feed(self, raw: bytes | str) -> None:
        self.pending += raw if isinstance(raw, str) else self.decoder.decode(raw)
        index = 0
        while index < len(self.pending):
            character = self.pending[index]
            if character != "\x1b":
                self.write(character)
                index += 1
                continue
            if index + 1 >= len(self.pending):
                break
            kind = self.pending[index + 1]
            if kind == "[":
                match = CSI.match(self.pending, index)
                if match is None:
                    break
                self.csi(match.group(1), match.group(3))
                index = match.end()
                continue
            if kind == "]":
                bell = self.pending.find("\x07", index + 2)
                string_term = self.pending.find("\x1b\\", index + 2)
                endings = [value for value in (bell, string_term) if value >= 0]
                if not endings:
                    break
                end = min(endings)
                index = end + (2 if self.pending[end : end + 2] == "\x1b\\" else 1)
                continue
            if kind in "()":
                if index + 2 >= len(self.pending):
                    break
                index += 3
                continue
            index += 2
        self.pending = self.pending[index:]

    def write(self, character: str) -> None:
        if character == "\r":
            self.x = 0
        elif character == "\n":
            self.y = min(self.height - 1, self.y + 1)
        elif character == "\b":
            self.x = max(0, self.x - 1)
        elif character == "\t":
            self.x = min(self.width - 1, (self.x // 8 + 1) * 8)
        elif ord(character) >= 0x20 and character != "\x7f":
            width = 0 if unicodedata.combining(character) else (2 if unicodedata.east_asian_width(character) in ("W", "F") else 1)
            if self.x < self.width and self.y < self.height:
                self.cells[self.y][self.x] = Cell(character, self.style)
                if width == 2 and self.x + 1 < self.width:
                    self.cells[self.y][self.x + 1] = Cell(" ", self.style)
            self.x = min(self.width - 1, self.x + width)

    @staticmethod
    def ansi_color(code: int, bright: bool = False) -> str:
        names = ["Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "Gray"]
        name = names[code]
        return ("Light" + name) if bright and name != "Black" else ("DarkGray" if bright else name)

    def sgr(self, values: list[int]) -> None:
        if not values:
            values = [0]
        foreground, background, bold = self.style.foreground, self.style.background, self.style.bold
        index = 0
        while index < len(values):
            value = values[index]
            if value == 0:
                foreground, background, bold = "Reset", "Reset", False
            elif value == 1:
                bold = True
            elif value == 22:
                bold = False
            elif 30 <= value <= 37:
                foreground = self.ansi_color(value - 30)
            elif 90 <= value <= 97:
                foreground = self.ansi_color(value - 90, True)
            elif value == 39:
                foreground = "Reset"
            elif 40 <= value <= 47:
                background = self.ansi_color(value - 40)
            elif 100 <= value <= 107:
                background = self.ansi_color(value - 100, True)
            elif value == 49:
                background = "Reset"
            elif value in (38, 48) and index + 1 < len(values):
                target = "foreground" if value == 38 else "background"
                if values[index + 1] == 2 and index + 4 < len(values):
                    color = f"Rgb({values[index + 2]}, {values[index + 3]}, {values[index + 4]})"
                    index += 4
                elif values[index + 1] == 5 and index + 2 < len(values):
                    color = f"Indexed({values[index + 2]})"
                    index += 2
                else:
                    color = "Reset"
                if target == "foreground":
                    foreground = color
                else:
                    background = color
            index += 1
        self.style = Style(foreground, background, bold)

    def erase(self, row: int, start: int, end: int) -> None:
        for column in range(max(0, start), min(self.width, end)):
            self.cells[row][column] = Cell(style=self.style)

    def csi(self, parameters: str, command: str) -> None:
        values = [int(value) if value else 0 for value in parameters.lstrip("?").split(";")]
        first = values[0] if values else 0
        amount = first or 1
        if command == "m":
            self.sgr(values)
        elif command in ("H", "f"):
            self.y = min(self.height - 1, max(0, (values[0] if values else 1) - 1))
            self.x = min(self.width - 1, max(0, (values[1] if len(values) > 1 else 1) - 1))
        elif command == "A": self.y = max(0, self.y - amount)
        elif command == "B": self.y = min(self.height - 1, self.y + amount)
        elif command == "C": self.x = min(self.width - 1, self.x + amount)
        elif command == "D": self.x = max(0, self.x - amount)
        elif command in ("G", "`"): self.x = min(self.width - 1, max(0, amount - 1))
        elif command == "d": self.y = min(self.height - 1, max(0, amount - 1))
        elif command == "J" and first in (2, 3): self.clear()
        elif command == "K":
            if first == 0: self.erase(self.y, self.x, self.width)
            elif first == 1: self.erase(self.y, 0, self.x + 1)
            elif first == 2: self.erase(self.y, 0, self.width)
        elif command == "s": self.saved = (self.x, self.y)
        elif command == "u": self.x, self.y = self.saved
