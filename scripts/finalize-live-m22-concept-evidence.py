#!/usr/bin/env python3
"""Render and index supported-host M22 live concept evidence."""

from __future__ import annotations

import argparse
import hashlib
import json
import struct
import subprocess
from pathlib import Path

try:
    import cairo
except ImportError as error:  # pragma: no cover - host prerequisite diagnostic
    raise SystemExit("PyCairo 1.27.0 is required for M22 live raster rendering") from error


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_EVIDENCE = ROOT / "artifacts/release-quality/m22-concept-live"
BASE_MANIFEST = ROOT / "artifacts/release-quality/next-generation-ui/manifest.json"
SCENARIOS = (
    "idle-dashboard",
    "active-build-tasks",
    "failed-build-errors",
    "rootfs-composition",
    "editor-application-menu",
    "terminal-sessions",
)
WIDTH = 160
HEIGHT = 50
CELL_WIDTH = 10
CELL_HEIGHT = 20
FONT_SIZE = 15.0
FONT_FAMILY = "DejaVu Sans Mono"
REGULAR_FONT = Path("/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf")
REGULAR_FONT_SHA256 = "a54dca07c76d6289e717e75e0a58c0128f6d7269ef3faf76417c9d7d3bba37ab"
BACKGROUND = (4 / 255, 12 / 255, 17 / 255)
FOREGROUND = (218 / 255, 222 / 255, 224 / 255)


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fail(message: str) -> None:
    raise SystemExit(f"M22 live evidence finalization failed: {message}")


def require_renderer() -> None:
    if cairo.version != "1.27.0" or cairo.cairo_version_string() != "1.18.4":
        fail(
            "expected PyCairo 1.27.0/Cairo 1.18.4, got "
            f"PyCairo {cairo.version}/Cairo {cairo.cairo_version_string()}"
        )
    if not REGULAR_FONT.is_file() or sha256(REGULAR_FONT) != REGULAR_FONT_SHA256:
        fail(f"pinned font is missing or changed: {REGULAR_FONT}")
    resolved = subprocess.run(
        ["fc-match", "-f", "%{file}", FONT_FAMILY],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    if Path(resolved).resolve() != REGULAR_FONT.resolve():
        fail(f"fontconfig resolved {FONT_FAMILY!r} to {resolved!r}")


def render(text_path: Path, output: Path) -> None:
    rows = text_path.read_text(encoding="utf-8").splitlines()
    if len(rows) > HEIGHT:
        fail(f"{text_path.name} has {len(rows)} rows, expected at most {HEIGHT}")
    surface = cairo.ImageSurface(
        cairo.FORMAT_RGB24, WIDTH * CELL_WIDTH, HEIGHT * CELL_HEIGHT
    )
    context = cairo.Context(surface)
    context.set_source_rgb(*BACKGROUND)
    context.paint()
    options = cairo.FontOptions()
    options.set_antialias(cairo.ANTIALIAS_GRAY)
    options.set_hint_metrics(cairo.HINT_METRICS_ON)
    options.set_hint_style(cairo.HINT_STYLE_FULL)
    context.set_font_options(options)
    context.select_font_face(FONT_FAMILY, cairo.FONT_SLANT_NORMAL, cairo.FONT_WEIGHT_NORMAL)
    context.set_font_size(FONT_SIZE)
    ascent, descent, _, _, _ = context.font_extents()
    context.set_source_rgb(*FOREGROUND)
    for row_index, row in enumerate(rows):
        for column, symbol in enumerate(row[:WIDTH]):
            if symbol.isspace():
                continue
            extents = context.text_extents(symbol)
            x = column * CELL_WIDTH + (CELL_WIDTH - extents.x_advance) / 2
            baseline = (
                row_index * CELL_HEIGHT
                + (CELL_HEIGHT - ascent - descent) / 2
                + ascent
            )
            context.move_to(x, baseline)
            context.show_text(symbol)
    surface.write_to_png(output)
    surface.finish()
    header = output.read_bytes()[:24]
    if (
        header[:8] != b"\x89PNG\r\n\x1a\n"
        or header[12:16] != b"IHDR"
        or struct.unpack(">II", header[16:24]) != (1600, 1000)
    ):
        fail(f"invalid live raster: {output}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--evidence", type=Path, default=DEFAULT_EVIDENCE)
    parser.add_argument("--base-manifest", type=Path, default=BASE_MANIFEST)
    args = parser.parse_args()
    require_renderer()
    evidence = args.evidence.resolve()
    if not evidence.is_dir():
        fail(f"missing evidence directory: {evidence}")
    base = json.loads(args.base_manifest.read_text(encoding="utf-8"))
    indexed: dict[str, dict[str, object]] = {}
    for scenario in SCENARIOS:
        report_path = evidence / f"{scenario}.report.json"
        text_path = evidence / f"{scenario}.txt"
        ansi_path = evidence / f"{scenario}.ansi"
        meta_path = evidence / f"{scenario}.meta"
        for path in (report_path, text_path, ansi_path, meta_path):
            if not path.is_file() or not path.stat().st_size:
                fail(f"missing {scenario} artifact: {path.name}")
        report = json.loads(report_path.read_text(encoding="utf-8"))
        if report.get("scenario") != scenario:
            fail(f"{report_path.name} has the wrong scenario identity")
        interactions = report.get("interactions")
        assertions = report.get("observed_assertions")
        if not isinstance(interactions, list) or not interactions or not all(
            isinstance(value, str) and value for value in interactions
        ):
            fail(f"{scenario} has no explicit interactions")
        if not isinstance(assertions, list) or not assertions or not all(
            isinstance(value, str) and value for value in assertions
        ):
            fail(f"{scenario} has no observed assertions")
        semantic = text_path.read_text(encoding="utf-8")
        auxiliary: Path | None = None
        if scenario == "terminal-sessions":
            auxiliary = evidence / "terminal-prefix-help.txt"
            if not auxiliary.is_file() or not auxiliary.stat().st_size:
                fail("terminal prefix-help semantic capture is missing")
            semantic += "\n" + auxiliary.read_text(encoding="utf-8")
        for assertion in assertions:
            if assertion not in semantic:
                fail(f"{scenario} semantic capture omitted assertion {assertion!r}")
        png_path = evidence / f"{scenario}.png"
        render(text_path, png_path)
        indexed[scenario] = {
            "report": report_path.name,
            "report_sha256": sha256(report_path),
            "terminal": ansi_path.name,
            "terminal_sha256": sha256(ansi_path),
            "semantic": text_path.name,
            "semantic_sha256": sha256(text_path),
            "metadata": meta_path.name,
            "metadata_sha256": sha256(meta_path),
            "raster": png_path.name,
            "raster_sha256": sha256(png_path),
            "interactions": interactions,
            "observed_assertions": assertions,
        }
        if auxiliary is not None:
            indexed[scenario]["auxiliary_semantic"] = auxiliary.name
            indexed[scenario]["auxiliary_semantic_sha256"] = sha256(auxiliary)
    manifest = {
        "schema": 1,
        "label": "supported-live-m22-concept-parity",
        "source_commit": base["source_commit"],
        "binary_sha256": base["binary_sha256"],
        "binary_profile": "dev",
        "host_distribution": base["host_distribution"],
        "host_libc": base["host_libc"],
        "poky_revision": base["poky_revision"],
        "poky_branch": base["poky_branch"],
        "bitbake_version": base["bitbake_version"],
        "machine": base["machine"],
        "distro": base["distro"],
        "yocto_release": base["yocto_release"],
        "target": base["target"],
        "started_utc": base["started_utc"],
        "finished_utc": base["finished_utc"],
        "raster_renderer": "yoctui-live-semantic-cairo-v1",
        "scenarios": indexed,
    }
    manifest_path = evidence / "manifest.json"
    manifest_path.write_text(
        json.dumps(manifest, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
    checksum_path = evidence / "checksums.sha256"
    files = sorted(
        path for path in evidence.iterdir() if path.is_file() and path != checksum_path
    )
    checksum_path.write_text(
        "".join(f"{sha256(path)}  {path.name}\n" for path in files), encoding="utf-8"
    )
    print(f"finalized {len(indexed)} supported-host M22 live scenarios at {evidence}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
