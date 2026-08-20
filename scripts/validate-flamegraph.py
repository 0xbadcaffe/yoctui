#!/usr/bin/env python3
"""Validate and summarize a generated Yoctui flamegraph."""

from __future__ import annotations

import argparse
import html
import re
from pathlib import Path


TITLE_RE = re.compile(r"<title>(.*?) \(([0-9,]+) samples(?:, [^)]+)?\)</title>")
TOTAL_RE = re.compile(r'total_samples="([0-9]+)"')
PERF_SAMPLES_RE = re.compile(r"Captured and wrote .* \(([0-9,]+) samples\)")
WORKLOAD_RE = re.compile(
    r"yoctui workbench profile: frames=([0-9]+) checksum=([0-9a-f]{16}) elapsed_ms=([0-9]+)"
)
UNRESOLVED = {"[unknown]", "unknown", "null", "(null)", "??"}
DIRECT_APPLICATION = {
    "layers",
    "matching_task_logs_for_task",
    "recipes",
    "render_at",
    "render_job_history",
    "render_task_log",
    "render_task_table",
    "update",
    "visible_task_row_refs_at",
    "workspace",
}


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("svg", type=Path)
    parser.add_argument("workload_log", type=Path)
    parser.add_argument("filter_report", type=Path)
    parser.add_argument("summary", type=Path)
    parser.add_argument("--minimum-samples", type=int, default=500)
    parser.add_argument("--minimum-frames", type=int, default=1_000)
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    svg = args.svg.read_text(encoding="utf-8")
    workload = args.workload_log.read_text(encoding="utf-8")
    filter_report = dict(
        line.split("=", 1)
        for line in args.filter_report.read_text(encoding="utf-8").splitlines()
    )
    if filter_report.get("schema") != "yoctui.flamegraph.filter.v1":
        raise SystemExit("flamegraph validation: stack-filter report is invalid")
    try:
        dropped_ppm = int(filter_report["dropped_unresolved_ppm"])
    except (KeyError, ValueError) as error:
        raise SystemExit(
            "flamegraph validation: unresolved-stack ratio is invalid"
        ) from error
    if not 0 <= dropped_ppm <= 5_000:
        raise SystemExit("flamegraph validation: unresolved-stack ratio exceeds 0.5%")
    total_match = TOTAL_RE.search(svg)
    if total_match is None:
        raise SystemExit("flamegraph validation: total_samples is missing")
    total_event_count = int(total_match.group(1))
    perf_samples_match = PERF_SAMPLES_RE.search(workload)
    if perf_samples_match is None:
        raise SystemExit("flamegraph validation: perf sample count is missing")
    recorded_samples = int(perf_samples_match.group(1).replace(",", ""))
    if recorded_samples < args.minimum_samples:
        raise SystemExit(
            f"flamegraph validation: {recorded_samples} samples is below "
            f"the {args.minimum_samples}-sample minimum"
        )

    workload_match = WORKLOAD_RE.search(workload)
    if workload_match is None:
        raise SystemExit("flamegraph validation: workload completion marker is missing")
    frames, checksum, elapsed_ms = workload_match.groups()
    if int(frames) < args.minimum_frames:
        raise SystemExit(
            f"flamegraph validation: {frames} frames is below "
            f"the {args.minimum_frames}-frame minimum"
        )
    if int(elapsed_ms) == 0:
        raise SystemExit("flamegraph validation: workload elapsed time is zero")

    parsed = [
        (html.unescape(name), int(samples.replace(",", "")))
        for name, samples in TITLE_RE.findall(svg)
    ]
    if not parsed:
        raise SystemExit("flamegraph validation: no stack frames were found")
    unresolved = sorted(
        {name for name, _ in parsed if name.strip().lower() in UNRESOLVED}
    )
    if unresolved:
        raise SystemExit(
            "flamegraph validation: unresolved/null frames found: "
            + ", ".join(unresolved)
        )
    application = [
        (name, samples)
        for name, samples in parsed
        if name in DIRECT_APPLICATION
        or name.startswith("yoctui_")
        or name.startswith("workbench_profile")
    ]
    if not application:
        raise SystemExit("flamegraph validation: no Yoctui application frames were found")

    ignored = {"all", "workbench_profile"}
    dominant: list[tuple[str, int]] = []
    seen: set[str] = set()
    for name, samples in sorted(application, key=lambda item: (-item[1], item[0])):
        if name in ignored or name in seen:
            continue
        seen.add(name)
        dominant.append((name, samples))
        if len(dominant) == 12:
            break
    if not dominant:
        raise SystemExit("flamegraph validation: dominant application symbols are missing")

    lines = [
        "schema=yoctui.flamegraph.summary.v1",
        f"workload_frames={frames}",
        f"workload_checksum={checksum}",
        f"workload_elapsed_ms={elapsed_ms}",
        f"total_samples={recorded_samples}",
        f"total_event_count={total_event_count}",
        "unresolved_frames=0",
        f"raw_unresolved_stack_lines={filter_report['unresolved_stack_lines']}",
        f"dropped_unresolved_event_count={filter_report['dropped_unresolved_event_count']}",
        f"dropped_unresolved_ppm={filter_report['dropped_unresolved_ppm']}",
    ]
    lines.extend(
        f"dominant_{index + 1}_samples={samples} dominant_{index + 1}_symbol={name}"
        for index, (name, samples) in enumerate(dominant)
    )
    args.summary.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(
        f"flamegraph valid: {recorded_samples} samples, {frames} frames, "
        f"checksum {checksum}, 0 unresolved frames"
    )


if __name__ == "__main__":
    main()
