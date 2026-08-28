#!/usr/bin/env python3
"""Verify real-Yoctui acceptance artifacts for the six M21 concept scenes."""

from __future__ import annotations

import hashlib
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


def verify_gap_task(
    task_id: object,
    scenario_id: str,
    context: str,
    implementation_tasks: set[str],
    open_gap_tasks: set[str],
    tasks: dict[str, dict[str, object]],
) -> None:
    if not isinstance(task_id, str) or task_id not in implementation_tasks:
        fail(f"{scenario_id}: {context} has invalid gap task {task_id!r}")
    if task_id not in open_gap_tasks:
        fail(f"{scenario_id}: {context} gap task {task_id} is not declared in open_gaps")
    if tasks[task_id]["status"] == "DONE":
        fail(f"{scenario_id}: completed task {task_id} still owns {context}")


def verify_external_evidence(
    evidence: object,
    kind: str,
    scenario_id: str,
    implementation_tasks: set[str],
    open_gap_tasks: set[str],
    tasks: dict[str, dict[str, object]],
    fixture_golden: Path | None = None,
) -> None:
    if not isinstance(evidence, dict):
        fail(f"{scenario_id}: {kind}_evidence must be a table")
    status = evidence.get("status")
    if status == "gap":
        verify_gap_task(
            evidence.get("task"),
            scenario_id,
            f"{kind} evidence",
            implementation_tasks,
            open_gap_tasks,
            tasks,
        )
        if set(evidence) != {"status", "task"}:
            fail(f"{scenario_id}: gap {kind}_evidence may contain only status and task")
        return
    if status != "verified":
        fail(f"{scenario_id}: {kind}_evidence status must be gap or verified")

    artifact = safe_repo_file(evidence.get("artifact"), scenario_id, f"{kind}_evidence.artifact")
    sha256 = evidence.get("sha256")
    if not isinstance(sha256, str) or not re.fullmatch(r"[0-9a-f]{64}", sha256):
        fail(f"{scenario_id}: verified {kind} evidence needs a SHA-256")
    if hashlib.sha256(artifact.read_bytes()).hexdigest() != sha256:
        fail(f"{scenario_id}: verified {kind} evidence checksum does not match")

    if kind == "raster":
        if artifact.suffix != ".png" or artifact.read_bytes()[:8] != b"\x89PNG\r\n\x1a\n":
            fail(f"{scenario_id}: verified raster evidence must be a PNG")
        source = safe_repo_file(
            evidence.get("source"), scenario_id, "raster_evidence.source"
        )
        if fixture_golden is None or source.resolve() != fixture_golden.resolve():
            fail(f"{scenario_id}: raster source must be the exact production cell golden")
        source_sha256 = evidence.get("source_sha256")
        if source_sha256 != hashlib.sha256(source.read_bytes()).hexdigest():
            fail(f"{scenario_id}: raster source checksum does not match")
        safe_repo_file(
            evidence.get("provenance"), scenario_id, "raster_evidence.provenance"
        )
        renderer = evidence.get("renderer")
        if not isinstance(renderer, str) or not renderer.strip():
            fail(f"{scenario_id}: raster evidence needs a pinned renderer identity")

    if kind == "live":
        interactions = evidence.get("interactions")
        assertions = evidence.get("assertions")
        if not isinstance(interactions, list) or not interactions or not all(
            isinstance(value, str) and value.strip() for value in interactions
        ):
            fail(f"{scenario_id}: verified live evidence needs explicit interactions")
        if not isinstance(assertions, list) or len(assertions) < 2 or not all(
            isinstance(value, str) and value.strip() for value in assertions
        ):
            fail(f"{scenario_id}: verified live evidence needs at least two assertions")
        artifact_text = artifact.read_text(encoding="utf-8")
        for assertion in assertions:
            if assertion not in artifact_text:
                fail(f"{scenario_id}: live evidence is missing assertion {assertion!r}")


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

        implementation_task_list = scenario.get("implementation_tasks")
        if not isinstance(implementation_task_list, list) or not implementation_task_list:
            fail(f"{scenario_id}: implementation_tasks must not be empty")
        if len(implementation_task_list) != len(set(implementation_task_list)):
            fail(f"{scenario_id}: implementation_tasks contains duplicates")
        implementation_tasks = set(implementation_task_list)
        for task_id in implementation_task_list:
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

        features = scenario.get("required_features")
        if not isinstance(features, list) or len(features) < 4:
            fail(f"{scenario_id}: at least four required_features are required")
        feature_ids: set[str] = set()
        feature_gap_tasks: set[str] = set()
        for feature in features:
            if not isinstance(feature, dict):
                fail(f"{scenario_id}: required feature must be a table")
            feature_id = feature.get("id")
            description = feature.get("description")
            if (
                not isinstance(feature_id, str)
                or not re.fullmatch(r"[a-z0-9]+(?:-[a-z0-9]+)*", feature_id)
                or feature_id in feature_ids
            ):
                fail(f"{scenario_id}: invalid or duplicate feature id {feature_id!r}")
            if not isinstance(description, str) or not description.strip():
                fail(f"{scenario_id}: feature {feature_id} needs a description")
            feature_ids.add(feature_id)
            fixture_anchors = feature.get("fixture_anchors")
            gap_task = feature.get("gap_task")
            if (fixture_anchors is None) == (gap_task is None):
                fail(
                    f"{scenario_id}: feature {feature_id} must have exactly one of "
                    "fixture_anchors or gap_task"
                )
            if gap_task is not None:
                verify_gap_task(
                    gap_task,
                    scenario_id,
                    f"feature {feature_id}",
                    implementation_tasks,
                    gap_tasks,
                    tasks,
                )
                feature_gap_tasks.add(gap_task)
                continue
            if (
                not isinstance(fixture_anchors, list)
                or not fixture_anchors
                or len(fixture_anchors) != len(set(fixture_anchors))
            ):
                fail(f"{scenario_id}: feature {feature_id} needs unique fixture anchors")
            for anchor in fixture_anchors:
                if not isinstance(anchor, str) or not anchor or anchor not in capture_text:
                    fail(
                        f"{scenario_id}: feature {feature_id} is missing fixture anchor "
                        f"{anchor!r}"
                    )
        if not feature_gap_tasks.issubset(gap_tasks):
            fail(f"{scenario_id}: feature gaps are not represented in open_gaps")

        verify_external_evidence(
            scenario.get("raster_evidence"),
            "raster",
            scenario_id,
            implementation_tasks,
            gap_tasks,
            tasks,
            golden,
        )
        verify_external_evidence(
            scenario.get("live_evidence"),
            "live",
            scenario_id,
            implementation_tasks,
            gap_tasks,
            tasks,
        )

    print(
        f"M21 concept screens verified: {len(scenarios)} production-renderer "
        f"scenes at {WIDTH}x{HEIGHT}; {gap_count} tracked implementation gaps"
    )


if __name__ == "__main__":
    main()
