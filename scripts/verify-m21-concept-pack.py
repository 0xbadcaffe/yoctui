#!/usr/bin/env python3
"""Verify integrity and metadata for the M21 visual concept pack."""

from __future__ import annotations

import hashlib
import struct
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
PACK = ROOT / "docs" / "design" / "m21" / "concepts"
MANIFEST = PACK / "manifest.toml"
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def fail(message: str) -> None:
    print(f"M21 concept pack verification failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def png_dimensions(path: Path) -> tuple[int, int]:
    with path.open("rb") as image:
        header = image.read(24)
    if len(header) != 24 or header[:8] != PNG_SIGNATURE or header[12:16] != b"IHDR":
        fail(f"{path.relative_to(ROOT)} is not a valid PNG with an IHDR header")
    return struct.unpack(">II", header[16:24])


def main() -> None:
    if not MANIFEST.is_file():
        fail(f"missing {MANIFEST.relative_to(ROOT)}")

    with MANIFEST.open("rb") as manifest_file:
        manifest = tomllib.load(manifest_file)

    if manifest.get("schema_version") != 1:
        fail("unsupported schema_version")
    if manifest.get("format") != "png":
        fail("concept format must be png")
    if manifest.get("exact_pixel_golden") is not False:
        fail("generated concepts must not be declared exact pixel goldens")

    expected_dimensions = (manifest.get("pixel_width"), manifest.get("pixel_height"))
    if not all(isinstance(value, int) and value > 0 for value in expected_dimensions):
        fail("pixel_width and pixel_height must be positive integers")

    scenarios = manifest.get("scenario")
    if not isinstance(scenarios, list) or len(scenarios) != 6:
        fail("manifest must declare exactly six scenarios")

    ids: set[str] = set()
    files: set[str] = set()
    for scenario in scenarios:
        scenario_id = scenario.get("id")
        filename = scenario.get("file")
        digest = scenario.get("sha256")
        anchors = scenario.get("anchors")

        if not isinstance(scenario_id, str) or not scenario_id:
            fail("every scenario needs a non-empty id")
        if scenario_id in ids:
            fail(f"duplicate scenario id {scenario_id}")
        ids.add(scenario_id)

        if not isinstance(filename, str) or Path(filename).name != filename:
            fail(f"scenario {scenario_id} has an unsafe file name")
        if not filename.endswith(".png"):
            fail(f"scenario {scenario_id} must reference a PNG")
        if filename in files:
            fail(f"duplicate scenario file {filename}")
        files.add(filename)

        if not isinstance(anchors, list) or len(anchors) < 3:
            fail(f"scenario {scenario_id} needs at least three review anchors")

        image_path = PACK / filename
        if not image_path.is_file():
            fail(f"missing {image_path.relative_to(ROOT)}")
        if png_dimensions(image_path) != expected_dimensions:
            fail(f"{filename} dimensions do not match the manifest")

        actual_digest = hashlib.sha256(image_path.read_bytes()).hexdigest()
        if actual_digest != digest:
            fail(f"{filename} SHA-256 does not match the manifest")

    actual_pngs = {path.name for path in PACK.glob("*.png")}
    if actual_pngs != files:
        fail(f"manifest/image mismatch: expected {sorted(files)}, found {sorted(actual_pngs)}")

    lossy_images = sorted(
        path.name
        for path in PACK.iterdir()
        if path.suffix.lower() in {".jpg", ".jpeg"}
    )
    if lossy_images:
        fail(f"lossy concept images are prohibited: {lossy_images}")

    print(
        f"M21 concept pack verified: {len(scenarios)} PNG scenarios "
        f"at {expected_dimensions[0]}x{expected_dimensions[1]}"
    )


if __name__ == "__main__":
    main()
