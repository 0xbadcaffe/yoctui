#!/usr/bin/env python3
"""Compare identical renderer fixtures without sampling or compiling in the window."""

from __future__ import annotations

import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import statistics
import subprocess
import sys


SCENARIOS = (
    "idle",
    "active-build",
    "large-metadata",
    "log-heavy",
    "telemetry",
    "menu-heavy",
    "large-rootfs",
    "large-graph",
    "large-editor",
    "dense-terminal",
)
RESULT = re.compile(
    r"^yoctui ui performance: scenario=(\S+) frames=(\d+) "
    r"checksum=([0-9a-f]{16}) elapsed_ms=(\d+) ns_per_frame=(\d+)$",
    re.M,
)


def measure(binary: Path, scenario: str, frames: int) -> tuple[str, int]:
    environment = os.environ.copy()
    environment.update(
        YOCTUI_PROFILE_SCENARIO=scenario, YOCTUI_PROFILE_FRAMES=str(frames)
    )
    output = subprocess.run(
        [str(binary)],
        env=environment,
        capture_output=True,
        text=True,
        timeout=120,
        check=True,
    ).stdout
    match = RESULT.search(output)
    if (
        not match
        or match[1] != scenario
        or int(match[2]) != frames
        or int(match[4]) <= 0
        or int(match[5]) <= 0
    ):
        raise ValueError(f"Invalid benchmark completion: {output[-1000:]}")
    return match[3], int(match[5])


def identity(binary: Path) -> dict[str, str]:
    with binary.open("rb") as source:
        digest = hashlib.file_digest(source, "sha256").hexdigest()
    return {"path": str(binary), "sha256": digest}


def compare(binaries: list[Path], scenario: str, frames: int, repeats: int) -> dict:
    for binary in binaries:
        measure(binary, scenario, 128)
    samples: list[list[int]] = [[], []]
    checksums: set[str] = set()
    for repetition in range(repeats):
        order = (0, 1) if repetition % 2 == 0 else (1, 0)
        for side in order:
            checksum, elapsed = measure(binaries[side], scenario, frames)
            checksums.add(checksum)
            samples[side].append(elapsed)
    if len(checksums) != 1:
        raise ValueError(
            f"Rendered output changed or was unstable in {scenario}: {checksums}"
        )
    before, after = map(statistics.median, samples)
    return {
        "checksum": checksums.pop(),
        "baseline_ns_per_frame": samples[0],
        "candidate_ns_per_frame": samples[1],
        "baseline_median_ns": before,
        "candidate_median_ns": after,
        "reduction_percent": round((before - after) * 100 / before, 2),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    parser.add_argument("--frames", type=int, default=1000)
    parser.add_argument("--repeats", type=int, default=5)
    parser.add_argument("--scenarios", nargs="+", choices=SCENARIOS, default=SCENARIOS)
    args = parser.parse_args()
    if args.frames < 128 or not 3 <= args.repeats <= 20:
        parser.error("Use at least 128 frames and 3..20 paired repeats")
    binaries = [args.baseline.resolve(strict=True), args.candidate.resolve(strict=True)]
    started_utc = datetime.now(timezone.utc).isoformat()
    results = {}
    for scenario in args.scenarios:
        # Warm each scenario; startup and fixture construction are outside the
        # Rust benchmark's reported rendering window, not timed as UI frames.
        result = compare(binaries, scenario, args.frames, args.repeats)
        results[scenario] = result
        before, after = result["baseline_median_ns"], result["candidate_median_ns"]
        print(
            f"{scenario}: median {before:.0f} → {after:.0f} ns/frame; identical checksum",
            file=sys.stderr,
            flush=True,
        )
    print(
        json.dumps(
            {
                "schema": "yoctui.renderer-comparison.v1",
                "captured_utc": datetime.now(timezone.utc).isoformat(),
                "started_utc": started_utc,
                "terminal": "160x48",
                "host": platform.node(),
                "platform": platform.platform(),
                "workload_sha256": identity(
                    Path(__file__).resolve().parents[1]
                    / "crates/yoctui-cli/benches/workbench_profile.rs"
                )["sha256"],
                "frames_per_run": args.frames,
                "paired_repeats": args.repeats,
                "warmup_frames": 128,
                "method": "sequential unsampled frame-pointer builds; alternating paired order; medians",
                "baseline": identity(binaries[0]),
                "candidate": identity(binaries[1]),
                "results": results,
            },
            indent=2,
        )
    )


if __name__ == "__main__":
    main()
