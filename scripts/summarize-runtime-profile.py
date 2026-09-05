#!/usr/bin/env python3
"""Create a concise, machine-readable report for a runtime perf capture."""

from __future__ import annotations

import argparse
import hashlib
import json
import platform
import re
from datetime import datetime, timezone
from pathlib import Path


SAMPLES = re.compile(r"Captured and wrote .* \(([0-9,]+) samples\)")
FLAT_ROW = re.compile(r"^\s*([0-9.]+)%\s+\S+\s+\S+\s+\[.\]\s+(.+?)\s*$")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def key_values(path: Path) -> dict[str, str]:
    values = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if "=" in line:
            key, value = line.split("=", 1)
            values[key] = value
    return values


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--scenario", required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--duration-seconds", required=True, type=int)
    parser.add_argument("--call-graph", required=True)
    parser.add_argument("--maximum-dropped-ppm", required=True, type=int)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--perf-log", required=True, type=Path)
    parser.add_argument("--flat-report", required=True, type=Path)
    parser.add_argument("--filter-report", required=True, type=Path)
    parser.add_argument("--svg", required=True, type=Path)
    parser.add_argument("--processes-json", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()

    perf_log = args.perf_log.read_text(encoding="utf-8")
    sample_match = SAMPLES.search(perf_log)
    if sample_match is None:
        raise SystemExit("runtime profile has no perf sample count")
    samples = int(sample_match.group(1).replace(",", ""))
    if samples < 50:
        raise SystemExit(f"runtime profile has only {samples} samples")
    quality = key_values(args.filter_report)
    if quality.get("schema") != "yoctui.flamegraph.filter.v1":
        raise SystemExit("runtime profile filter report is invalid")
    dropped_ppm = int(quality["dropped_unresolved_ppm"])
    if dropped_ppm > args.maximum_dropped_ppm:
        raise SystemExit("runtime profile unresolved-frame ratio exceeds its quality ceiling")

    top_symbols = []
    for line in args.flat_report.read_text(encoding="utf-8").splitlines():
        match = FLAT_ROW.match(line)
        if match:
            top_symbols.append(
                {"self_percent": float(match.group(1)), "symbol": match.group(2)}
            )
            if len(top_symbols) == 20:
                break
    if not any("yoctui" in entry["symbol"] for entry in top_symbols):
        raise SystemExit("runtime profile has no resolved Yoctui symbols")

    binary = args.binary.resolve(strict=True)
    record = {
        "schema": "yoctui.performance.runtime-profile.v1",
        "captured_at_utc": datetime.now(timezone.utc).isoformat(),
        "scenario": args.scenario,
        "revision": args.revision,
        "host": {"kernel": platform.release(), "machine": platform.machine()},
        "binary": {"path": str(binary), "sha256": sha256(binary)},
        "sampling": {
            "event": "cycles:u",
            "frequency_hz": 499,
            "call_graph": args.call_graph,
            "duration_seconds": args.duration_seconds,
            "samples": samples,
            "unresolved_stack_lines": int(quality["unresolved_stack_lines"]),
            "dropped_unresolved_ppm": dropped_ppm,
            "maximum_dropped_unresolved_ppm": args.maximum_dropped_ppm,
        },
        "processes": json.loads(args.processes_json.read_text(encoding="utf-8")),
        "artifacts": {
            "svg_sha256": sha256(args.svg),
            "flat_report_sha256": sha256(args.flat_report),
        },
        "top_self_symbols": top_symbols,
    }
    args.output.write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
    print(
        f"runtime profile valid: {args.scenario}, {samples} samples, "
        f"{dropped_ppm} unresolved ppm"
    )


if __name__ == "__main__":
    main()
