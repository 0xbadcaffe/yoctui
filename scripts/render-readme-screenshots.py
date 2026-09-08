#!/usr/bin/env python3
"""Render the README gallery from exact production TestBackend cell goldens."""

from __future__ import annotations

import argparse
import runpy
import tempfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUTPUT_DIR = ROOT / "docs/media/screenshots"
PROVENANCE = OUTPUT_DIR / "manifest.toml"
RASTER = runpy.run_path(str(ROOT / "scripts/render-m22-concept-screenshots.py"))
SCENARIOS = [
    ("active-build-tasks", "concept-active-build-tasks-160x50.cells"),
    ("kernel-device-tree", "readme-kernel-device-tree-160x50.cells"),
    ("uboot-device-tree", "readme-uboot-device-tree-160x50.cells"),
    ("kernel-menuconfig", "readme-kernel-menuconfig-160x50.cells"),
    ("uboot-menuconfig", "readme-uboot-menuconfig-160x50.cells"),
    ("rootfs-composition", "concept-rootfs-composition-160x50.cells"),
    ("idle-dashboard", "concept-idle-dashboard-160x50.cells"),
    ("failed-build-errors", "concept-failed-build-errors-160x50.cells"),
    ("editor-application-menu", "concept-editor-application-menu-160x50.cells"),
    ("terminal-sessions", "concept-terminal-sessions-160x50.cells"),
]


def fail(message: str) -> None:
    raise SystemExit(f"README screenshot gallery failed: {message}")


def source_path(name: str) -> Path:
    return ROOT / "crates/yoctui-ui/tests/golden" / name


def render_all(output_root: Path) -> list[tuple[str, Path, Path]]:
    rendered = []
    for index, (scenario_id, source_name) in enumerate(SCENARIOS, start=1):
        source = source_path(source_name)
        if not source.is_file() or source.suffix != ".cells":
            fail(f"missing exact production cell golden for {scenario_id}")
        artifact = output_root / f"{index:02d}-{scenario_id}.png"
        RASTER["render_cell_golden"](source, artifact)
        expected_dimensions = (
            RASTER["WIDTH"] * RASTER["CELL_WIDTH"],
            RASTER["HEIGHT"] * RASTER["CELL_HEIGHT"],
        )
        if RASTER["png_dimensions"](artifact) != expected_dimensions:
            fail(f"wrong PNG dimensions for {scenario_id}")
        rendered.append((scenario_id, source, artifact))
    return rendered


def provenance_text(rendered: list[tuple[str, Path, Path]]) -> str:
    lines = [
        "schema_version = 1",
        'renderer = "yoctui-cairo-cell-raster-v1"',
        'authority = "production TestBackend cell/style goldens"',
        f'pycairo_version = "{RASTER["PYCAIRO_VERSION"]}"',
        f'cairo_version = "{RASTER["CAIRO_VERSION"]}"',
        f'font_family = "{RASTER["FONT_FAMILY"]}"',
        f'regular_font_sha256 = "{RASTER["REGULAR_FONT_SHA256"]}"',
        f'bold_font_sha256 = "{RASTER["BOLD_FONT_SHA256"]}"',
        f'logical_columns = {RASTER["WIDTH"]}',
        f'logical_rows = {RASTER["HEIGHT"]}',
        f'pixel_width = {RASTER["WIDTH"] * RASTER["CELL_WIDTH"]}',
        f'pixel_height = {RASTER["HEIGHT"] * RASTER["CELL_HEIGHT"]}',
        "",
    ]
    for scenario_id, source, artifact in rendered:
        destination = OUTPUT_DIR / artifact.name
        lines.extend(
            [
                "[[artifact]]",
                f'id = "{scenario_id}"',
                f'source = "{source.relative_to(ROOT)}"',
                f'source_sha256 = "{RASTER["sha256"](source)}"',
                f'file = "{destination.relative_to(ROOT)}"',
                f'sha256 = "{RASTER["sha256"](artifact)}"',
                "",
            ]
        )
    return "\n".join(lines)


def update() -> None:
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    rendered = render_all(OUTPUT_DIR)
    PROVENANCE.write_text(provenance_text(rendered), encoding="utf-8")
    print(f"README screenshots updated: {len(rendered)} deterministic PNGs")


def check() -> None:
    if not PROVENANCE.is_file():
        fail(f"missing {PROVENANCE.relative_to(ROOT)}; run with --update")
    expected_names = {
        f"{index:02d}-{scenario_id}.png"
        for index, (scenario_id, _) in enumerate(SCENARIOS, start=1)
    }
    actual_names = {path.name for path in OUTPUT_DIR.glob("*.png")}
    if actual_names != expected_names:
        fail("gallery PNG set or ordering is stale; run with --update")
    with tempfile.TemporaryDirectory(prefix="yoctui-readme-gallery-") as temporary:
        rendered = render_all(Path(temporary))
        if PROVENANCE.read_text(encoding="utf-8") != provenance_text(rendered):
            fail("gallery provenance is stale; run with --update")
        for _, _, actual in rendered:
            expected = OUTPUT_DIR / actual.name
            if expected.read_bytes() != actual.read_bytes():
                fail(f"deterministic screenshot is stale: {expected.relative_to(ROOT)}")
    print(f"README screenshots verified: {len(SCENARIOS)} deterministic PNGs")


def main() -> None:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--check", action="store_true")
    mode.add_argument("--update", action="store_true")
    arguments = parser.parse_args()
    RASTER["require_renderer"]()
    update() if arguments.update else check()


if __name__ == "__main__":
    main()
