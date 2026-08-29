#!/usr/bin/env python3
"""Render deterministic PNG review artifacts from exact Yoctui cell goldens."""

from __future__ import annotations

import argparse
import hashlib
import re
import struct
import subprocess
import sys
import tempfile
import tomllib
from dataclasses import dataclass
from pathlib import Path

try:
    import cairo
except ImportError as error:  # pragma: no cover - environment diagnostic
    raise SystemExit("PyCairo 1.27.0 is required for M22 raster rendering") from error


ROOT = Path(__file__).resolve().parents[1]
CONCEPT_MANIFEST = ROOT / "docs/design/m21/concepts/manifest.toml"
OUTPUT_DIR = ROOT / "docs/design/m22/production-raster"
PROVENANCE = OUTPUT_DIR / "manifest.toml"
WIDTH = 160
HEIGHT = 50
CELL_WIDTH = 10
CELL_HEIGHT = 20
FONT_SIZE = 15.0
PYCAIRO_VERSION = "1.27.0"
CAIRO_VERSION = "1.18.4"
FONT_FAMILY = "DejaVu Sans Mono"
REGULAR_FONT = Path("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf")
BOLD_FONT = Path("/usr/share/fonts/truetype/dejavu/DejaVuSansMono-Bold.ttf")
REGULAR_FONT_SHA256 = "a54dca07c76d6289e717e75e0a58c0128f6d7269ef3faf76417c9d7d3bba37ab"
BOLD_FONT_SHA256 = "2d67eed9b325ee2e69f906ce4f3f6264d5105230a13352c8494daad4f17b12c8"
STYLE_RE = re.compile(
    r"^T\|(\d+)\|fg=([^;]+);bg=([^;]+);ul=([^;]+);mod=([^;]+)$"
)
RGB_RE = re.compile(r"^Rgb\((\d+), (\d+), (\d+)\)$")
INDEXED_RE = re.compile(r"^Indexed\((\d+)\)$")
NAMED_COLORS = {
    "Black": (0, 0, 0),
    "Blue": (0, 0, 255),
    "Cyan": (0, 255, 255),
    "DarkGray": (128, 128, 128),
    "Gray": (128, 128, 128),
    "Green": (0, 255, 0),
    "LightBlue": (128, 128, 255),
    "LightCyan": (128, 255, 255),
    "LightGreen": (128, 255, 128),
    "LightMagenta": (255, 128, 255),
    "LightRed": (255, 128, 128),
    "LightYellow": (255, 255, 128),
    "Magenta": (255, 0, 255),
    "Red": (255, 0, 0),
    "White": (255, 255, 255),
    "Yellow": (255, 255, 0),
}
DEFAULT_FOREGROUND = (218, 222, 224)
DEFAULT_BACKGROUND = (4, 12, 17)


@dataclass(frozen=True)
class CellStyle:
    foreground: tuple[int, int, int]
    background: tuple[int, int, int]
    bold: bool


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fail(message: str) -> None:
    raise SystemExit(f"M22 concept raster failed: {message}")


def require_renderer() -> None:
    if cairo.version != PYCAIRO_VERSION or cairo.cairo_version_string() != CAIRO_VERSION:
        fail(
            "renderer version mismatch: expected "
            f"PyCairo {PYCAIRO_VERSION}/Cairo {CAIRO_VERSION}, got "
            f"PyCairo {cairo.version}/Cairo {cairo.cairo_version_string()}"
        )
    for path, expected in [
        (REGULAR_FONT, REGULAR_FONT_SHA256),
        (BOLD_FONT, BOLD_FONT_SHA256),
    ]:
        if not path.is_file() or sha256(path) != expected:
            fail(f"pinned font is missing or changed: {path}")
    for pattern, expected in [
        (FONT_FAMILY, REGULAR_FONT.resolve()),
        (f"{FONT_FAMILY}:style=Bold", BOLD_FONT.resolve()),
    ]:
        result = subprocess.run(
            ["fc-match", "-f", "%{file}", pattern],
            check=True,
            capture_output=True,
            text=True,
        )
        if Path(result.stdout).resolve() != expected:
            fail(f"fontconfig resolved {pattern!r} to {result.stdout!r}, expected {expected}")


def parse_color(value: str, default: tuple[int, int, int]) -> tuple[int, int, int]:
    if value == "Reset":
        return default
    match = RGB_RE.fullmatch(value)
    if match:
        color = tuple(int(component) for component in match.groups())
        if all(0 <= component <= 255 for component in color):
            return color  # type: ignore[return-value]
    if value in NAMED_COLORS:
        return NAMED_COLORS[value]
    indexed = INDEXED_RE.fullmatch(value)
    if indexed:
        index = int(indexed.group(1))
        if index < 16:
            return (
                (0, 0, 0),
                (128, 0, 0),
                (0, 128, 0),
                (128, 128, 0),
                (0, 0, 128),
                (128, 0, 128),
                (0, 128, 128),
                (192, 192, 192),
                (128, 128, 128),
                (255, 0, 0),
                (0, 255, 0),
                (255, 255, 0),
                (0, 0, 255),
                (255, 0, 255),
                (0, 255, 255),
                (255, 255, 255),
            )[index]
        if index < 232:
            cube = index - 16
            levels = (0, 95, 135, 175, 215, 255)
            return (
                levels[cube // 36],
                levels[(cube // 6) % 6],
                levels[cube % 6],
            )
        level = 8 + (index - 232) * 10
        return (level, level, level)
    fail(f"unsupported cell color {value!r}")


def parse_symbol_row(line: str) -> list[str]:
    if not line.startswith("S|"):
        fail("symbol row does not start with S|")
    data = line[2:].encode("utf-8")
    cursor = 0
    output: list[str] = []
    while cursor < len(data):
        colon = data.find(b":", cursor)
        if colon < 0:
            fail("symbol byte length has no colon")
        try:
            length = int(data[cursor:colon])
        except ValueError:
            fail("symbol byte length is not numeric")
        cursor = colon + 1
        end = cursor + length
        if length <= 0 or end > len(data):
            fail("symbol byte length exceeds its row")
        try:
            output.append(data[cursor:end].decode("utf-8"))
        except UnicodeDecodeError as error:
            fail(f"symbol is not valid UTF-8: {error}")
        cursor = end
    return output


def parse_cell_golden(path: Path) -> tuple[list[str], list[CellStyle]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    if lines[:2] != [f"YOCTUI_CELL_GOLDEN_V1 {WIDTH} {HEIGHT}", "SYMBOLS"]:
        fail(f"invalid cell header in {path.relative_to(ROOT)}")
    if len(lines) < HEIGHT + 3 or lines[HEIGHT + 2] != "STYLES":
        fail(f"missing bounded STYLES section in {path.relative_to(ROOT)}")
    symbols: list[str] = []
    for row in lines[2 : HEIGHT + 2]:
        parsed = parse_symbol_row(row)
        if len(parsed) != WIDTH:
            fail(f"symbol row has {len(parsed)} cells, expected {WIDTH}")
        symbols.extend(parsed)
    styles: list[CellStyle] = []
    for line in lines[HEIGHT + 3 :]:
        match = STYLE_RE.fullmatch(line)
        if not match:
            fail(f"malformed style run {line!r}")
        count, foreground, background, underline, modifiers = match.groups()
        if underline != "Reset" or modifiers not in {"NONE", "BOLD"}:
            fail(f"unsupported style projection {line!r}")
        style = CellStyle(
            foreground=parse_color(foreground, DEFAULT_FOREGROUND),
            background=parse_color(background, DEFAULT_BACKGROUND),
            bold=modifiers == "BOLD",
        )
        styles.extend([style] * int(count))
    expected = WIDTH * HEIGHT
    if len(symbols) != expected or len(styles) != expected:
        fail(
            f"cell coverage mismatch in {path.relative_to(ROOT)}: "
            f"symbols={len(symbols)} styles={len(styles)} expected={expected}"
        )
    return symbols, styles


def set_rgb(context: cairo.Context, color: tuple[int, int, int]) -> None:
    context.set_source_rgb(*(component / 255.0 for component in color))


def render_cell_golden(source: Path, destination: Path) -> None:
    symbols, styles = parse_cell_golden(source)
    surface = cairo.ImageSurface(
        cairo.FORMAT_RGB24, WIDTH * CELL_WIDTH, HEIGHT * CELL_HEIGHT
    )
    context = cairo.Context(surface)
    font_options = cairo.FontOptions()
    font_options.set_antialias(cairo.ANTIALIAS_GRAY)
    font_options.set_hint_metrics(cairo.HINT_METRICS_ON)
    font_options.set_hint_style(cairo.HINT_STYLE_FULL)
    context.set_font_options(font_options)

    for index, style in enumerate(styles):
        column = index % WIDTH
        row = index // WIDTH
        set_rgb(context, style.background)
        context.rectangle(
            column * CELL_WIDTH, row * CELL_HEIGHT, CELL_WIDTH, CELL_HEIGHT
        )
        context.fill()

    current_bold: bool | None = None
    for index, (symbol, style) in enumerate(zip(symbols, styles, strict=True)):
        if symbol.isspace() or not symbol:
            continue
        if current_bold != style.bold:
            context.select_font_face(
                FONT_FAMILY,
                cairo.FONT_SLANT_NORMAL,
                cairo.FONT_WEIGHT_BOLD if style.bold else cairo.FONT_WEIGHT_NORMAL,
            )
            context.set_font_size(FONT_SIZE)
            current_bold = style.bold
        extents = context.text_extents(symbol)
        ascent, descent, _, _, _ = context.font_extents()
        column = index % WIDTH
        row = index // WIDTH
        x = column * CELL_WIDTH + (CELL_WIDTH - extents.x_advance) / 2
        baseline = (
            row * CELL_HEIGHT
            + (CELL_HEIGHT - ascent - descent) / 2
            + ascent
        )
        set_rgb(context, style.foreground)
        context.move_to(x, baseline)
        context.show_text(symbol)

    destination.parent.mkdir(parents=True, exist_ok=True)
    surface.write_to_png(destination)
    surface.finish()


def png_dimensions(path: Path) -> tuple[int, int]:
    header = path.read_bytes()[:24]
    if len(header) != 24 or header[:8] != b"\x89PNG\r\n\x1a\n" or header[12:16] != b"IHDR":
        fail(f"renderer did not create a valid PNG: {path}")
    return struct.unpack(">II", header[16:24])


def scenarios() -> list[tuple[str, Path]]:
    with CONCEPT_MANIFEST.open("rb") as manifest_file:
        manifest = tomllib.load(manifest_file)
    output: list[tuple[str, Path]] = []
    for scenario in manifest.get("scenario", []):
        scenario_id = scenario.get("id")
        source_value = scenario.get("real_yoctui_golden")
        if not isinstance(scenario_id, str) or not isinstance(source_value, str):
            fail("concept manifest has an invalid scenario identity or cell golden")
        source = ROOT / source_value
        if not source.is_file() or source.suffix != ".cells":
            fail(f"missing exact cell golden for {scenario_id}")
        output.append((scenario_id, source))
    if len(output) != 6:
        fail(f"expected six scenarios, found {len(output)}")
    return output


def provenance_text(
    rendered: list[tuple[str, Path, Path]], output_root: Path
) -> str:
    lines = [
        "schema_version = 1",
        'renderer = "yoctui-cairo-cell-raster-v1"',
        f'pycairo_version = "{PYCAIRO_VERSION}"',
        f'cairo_version = "{CAIRO_VERSION}"',
        f'font_family = "{FONT_FAMILY}"',
        f'regular_font_sha256 = "{REGULAR_FONT_SHA256}"',
        f'bold_font_sha256 = "{BOLD_FONT_SHA256}"',
        f"logical_columns = {WIDTH}",
        f"logical_rows = {HEIGHT}",
        f"cell_width = {CELL_WIDTH}",
        f"cell_height = {CELL_HEIGHT}",
        f"pixel_width = {WIDTH * CELL_WIDTH}",
        f"pixel_height = {HEIGHT * CELL_HEIGHT}",
        'antialias = "gray"',
        'hint_style = "full"',
        'hint_metrics = "on"',
        "",
    ]
    for scenario_id, source, artifact in rendered:
        stored_artifact = OUTPUT_DIR / artifact.name
        lines.extend(
            [
                "[[artifact]]",
                f'id = "{scenario_id}"',
                f'source = "{source.relative_to(ROOT)}"',
                f'source_sha256 = "{sha256(source)}"',
                f'file = "{stored_artifact.relative_to(ROOT)}"',
                f'sha256 = "{sha256(artifact)}"',
                "",
            ]
        )
    return "\n".join(lines)


def render_all(output_root: Path) -> list[tuple[str, Path, Path]]:
    rendered = []
    for index, (scenario_id, source) in enumerate(scenarios(), start=1):
        artifact = output_root / f"{index:02d}-{scenario_id}.png"
        render_cell_golden(source, artifact)
        if png_dimensions(artifact) != (WIDTH * CELL_WIDTH, HEIGHT * CELL_HEIGHT):
            fail(f"wrong PNG dimensions for {scenario_id}")
        rendered.append((scenario_id, source, artifact))
    return rendered


def update() -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    rendered = render_all(OUTPUT_DIR)
    PROVENANCE.write_text(provenance_text(rendered, OUTPUT_DIR), encoding="utf-8")
    print(f"M22 production rasters updated: {len(rendered)} deterministic PNGs")


def check() -> None:
    if not PROVENANCE.is_file():
        fail(f"missing {PROVENANCE.relative_to(ROOT)}; run with --update")
    with tempfile.TemporaryDirectory(prefix="yoctui-m22-raster-") as temporary:
        temporary_root = Path(temporary)
        rendered = render_all(temporary_root)
        expected_provenance = provenance_text(rendered, temporary_root)
        if PROVENANCE.read_text(encoding="utf-8") != expected_provenance:
            fail("raster provenance is stale; run with --update")
        for _, _, actual in rendered:
            expected = OUTPUT_DIR / actual.name
            if not expected.is_file() or expected.read_bytes() != actual.read_bytes():
                fail(f"deterministic raster is missing or stale: {expected.relative_to(ROOT)}")
    print(f"M22 production rasters verified: {len(rendered)} deterministic PNGs")


def main() -> None:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--update", action="store_true")
    arguments = parser.parse_args()
    require_renderer()
    if arguments.update:
        update()
    else:
        check()


if __name__ == "__main__":
    main()
