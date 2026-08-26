#!/usr/bin/env python3
"""Verify real-Yoctui acceptance artifacts for the six M21 concept scenes."""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs" / "design" / "m21" / "concepts" / "manifest.toml"
REGISTRY = ROOT / "docs" / "task-registry.toml"
WIDTH = 160
HEIGHT = 50
CELL_COUNT = WIDTH * HEIGHT


def fail(message: str) -> None:
    print(f"M21 concept screen verification failed: {message}", file=sys.stderr)
    raise SystemExit(1)


def safe_repo_file(value: object, scenario_id: str, field: str) -> Path:
    if not isinstance(value, str) or not value:
        fail(f"{scenario_id}: {field} must be a non-empty repository path")
    relative = Path(value)
    if relative.is_absolute() or ".." in relative.parts:
        fail(f"{scenario_id}: {field} must stay inside the repository")
    path = ROOT / relative
    if not path.is_file():
        fail(f"{scenario_id}: missing {relative}")
    return path


def parse_symbol_row(line: str, scenario_id: str) -> int:
    if not line.startswith("S|"):
        fail(f"{scenario_id}: golden symbol row does not start with S|")
    data = line[2:].encode()
    cursor = 0
    symbols = 0
    while cursor < len(data):
        colon = data.find(b":", cursor)
        if colon < 0:
            fail(f"{scenario_id}: golden symbol length has no colon")
        try:
            length = int(data[cursor:colon])
        except ValueError:
            fail(f"{scenario_id}: golden symbol length is not numeric")
        cursor = colon + 1 + length
        if cursor > len(data):
            fail(f"{scenario_id}: golden symbol exceeds its row")
        symbols += 1
    return symbols


def verify_cell_golden(path: Path, scenario_id: str) -> None:
    lines = path.read_text(encoding="utf-8").splitlines()
    expected_header = f"YOCTUI_CELL_GOLDEN_V1 {WIDTH} {HEIGHT}"
    if lines[:2] != [expected_header, "SYMBOLS"]:
        fail(f"{scenario_id}: invalid cell-golden header")
    if len(lines) < HEIGHT + 3 or lines[HEIGHT + 2] != "STYLES":
        fail(f"{scenario_id}: cell golden has no bounded STYLES section")
    for row, line in enumerate(lines[2 : HEIGHT + 2]):
        symbols = parse_symbol_row(line, scenario_id)
        if symbols != WIDTH:
            fail(f"{scenario_id}: symbol row {row} has {symbols} cells, expected {WIDTH}")

    style_cells = 0
    for line in lines[HEIGHT + 3 :]:
        match = re.match(r"^T\|(\d+)\|fg=.*;bg=.*;ul=.*;mod=.*$", line)
        if not match:
            fail(f"{scenario_id}: malformed style run {line!r}")
        style_cells += int(match.group(1))
    if style_cells != CELL_COUNT:
        fail(f"{scenario_id}: styles cover {style_cells} cells, expected {CELL_COUNT}")


def main() -> None:
    with MANIFEST.open("rb") as manifest_file:
        manifest = tomllib.load(manifest_file)
    with REGISTRY.open("rb") as registry_file:
        registry = tomllib.load(registry_file)

    tasks = {task["id"]: task for task in registry.get("task", [])}
    scenarios = manifest.get("scenario")
    if not isinstance(scenarios, list) or len(scenarios) != 6:
        fail("manifest must contain exactly six scenarios")
    if manifest.get("exact_pixel_golden") is not False:
        fail("generated concepts must remain non-authoritative for exact pixels")

    ids: set[str] = set()
    golden_paths: set[Path] = set()
    capture_paths: set[Path] = set()
    gap_count = 0
    for scenario in scenarios:
        scenario_id = scenario.get("id")
        if not isinstance(scenario_id, str) or not scenario_id or scenario_id in ids:
            fail(f"invalid or duplicate scenario id {scenario_id!r}")
        ids.add(scenario_id)

        golden = safe_repo_file(
            scenario.get("real_yoctui_golden"), scenario_id, "real_yoctui_golden"
        )
        capture = safe_repo_file(
            scenario.get("real_yoctui_capture"), scenario_id, "real_yoctui_capture"
        )
        if golden.suffix != ".cells" or golden in golden_paths:
            fail(f"{scenario_id}: golden must be a unique .cells file")
        if capture.suffix != ".txt" or capture in capture_paths:
            fail(f"{scenario_id}: capture must be a unique .txt file")
        golden_paths.add(golden)
        capture_paths.add(capture)
        verify_cell_golden(golden, scenario_id)

        capture_text = capture.read_text(encoding="utf-8")
        if len(capture_text.splitlines()) != HEIGHT:
            fail(f"{scenario_id}: semantic capture must contain exactly {HEIGHT} rows")
        anchors = scenario.get("real_yoctui_anchors")
        if not isinstance(anchors, list) or len(anchors) < 4:
            fail(f"{scenario_id}: at least four real-Yoctui anchors are required")
        for anchor in anchors:
            if not isinstance(anchor, str) or not anchor or anchor not in capture_text:
                fail(f"{scenario_id}: semantic capture is missing anchor {anchor!r}")

        implementation_tasks = scenario.get("implementation_tasks")
        if not isinstance(implementation_tasks, list) or not implementation_tasks:
            fail(f"{scenario_id}: implementation_tasks must not be empty")
        if len(implementation_tasks) != len(set(implementation_tasks)):
            fail(f"{scenario_id}: implementation_tasks contains duplicates")
        for task_id in implementation_tasks:
            if task_id not in tasks:
                fail(f"{scenario_id}: unknown implementation task {task_id}")

        gaps = scenario.get("open_gaps")
        if not isinstance(gaps, list):
            fail(f"{scenario_id}: open_gaps must be a list")
        gap_tasks: set[str] = set()
        for gap in gaps:
            task_id = gap.get("task") if isinstance(gap, dict) else None
            description = gap.get("description") if isinstance(gap, dict) else None
            if task_id not in implementation_tasks or task_id in gap_tasks:
                fail(f"{scenario_id}: invalid or duplicate gap task {task_id!r}")
            if not isinstance(description, str) or not description.strip():
                fail(f"{scenario_id}: gap {task_id} needs a description")
            if tasks[task_id]["status"] == "DONE":
                fail(f"{scenario_id}: completed task {task_id} still owns an open gap")
            gap_tasks.add(task_id)
            gap_count += 1

    print(
        f"M21 concept screens verified: {len(scenarios)} production-renderer "
        f"scenes at {WIDTH}x{HEIGHT}; {gap_count} tracked implementation gaps"
    )


if __name__ == "__main__":
    main()
