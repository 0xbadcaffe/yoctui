#!/usr/bin/env python3
"""Remove rare corrupt perf call chains before flamegraph rendering."""

from __future__ import annotations

import os
import sys
from pathlib import Path


UNRESOLVED = {"[unknown]", "unknown", "null", "(null)", "??"}
MAX_DROPPED_PPM = 5_000


def main() -> None:
    report_value = os.environ.get("YOCTUI_FLAMEGRAPH_FILTER_REPORT")
    if not report_value:
        raise SystemExit("flamegraph stack filter: report path is missing")

    raw_lines = 0
    accepted_lines = 0
    unresolved_lines = 0
    raw_weight = 0
    dropped_weight = 0
    for line in sys.stdin:
        stack, separator, weight_text = line.rstrip("\n").rpartition(" ")
        if not separator:
            raise SystemExit("flamegraph stack filter: malformed folded stack")
        try:
            weight = int(weight_text)
        except ValueError as error:
            raise SystemExit(
                "flamegraph stack filter: malformed folded stack weight"
            ) from error
        raw_lines += 1
        raw_weight += weight
        if any(frame.strip().lower() in UNRESOLVED for frame in stack.split(";")):
            unresolved_lines += 1
            dropped_weight += weight
            continue
        accepted_lines += 1
        sys.stdout.write(line)

    if raw_weight == 0 or accepted_lines == 0:
        raise SystemExit("flamegraph stack filter: no resolved samples remain")
    dropped_ppm = dropped_weight * 1_000_000 // raw_weight
    if dropped_ppm > MAX_DROPPED_PPM:
        raise SystemExit(
            "flamegraph stack filter: unresolved call chains are "
            f"{dropped_ppm} ppm, above the {MAX_DROPPED_PPM} ppm limit"
        )

    Path(report_value).write_text(
        "\n".join(
            [
                "schema=yoctui.flamegraph.filter.v1",
                f"raw_stack_lines={raw_lines}",
                f"accepted_stack_lines={accepted_lines}",
                f"unresolved_stack_lines={unresolved_lines}",
                f"raw_event_count={raw_weight}",
                f"dropped_unresolved_event_count={dropped_weight}",
                f"dropped_unresolved_ppm={dropped_ppm}",
            ]
        )
        + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
