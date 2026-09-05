#!/usr/bin/env python3
"""Remove rare corrupt perf call chains before flamegraph rendering."""

from __future__ import annotations

import os
import sys
from pathlib import Path


UNRESOLVED = {"[unknown]", "unknown", "null", "(null)", "??"}
DEFAULT_MAX_DROPPED_PPM = 5_000


def main() -> None:
    report_value = os.environ.get("YOCTUI_FLAMEGRAPH_FILTER_REPORT")
    if not report_value:
        raise SystemExit("flamegraph stack filter: report path is missing")

    try:
        maximum_dropped_ppm = int(
            os.environ.get(
                "YOCTUI_FLAMEGRAPH_MAX_DROPPED_PPM", str(DEFAULT_MAX_DROPPED_PPM)
            )
        )
    except ValueError as error:
        raise SystemExit("flamegraph stack filter: invalid quality ceiling") from error
    if not 0 <= maximum_dropped_ppm <= 50_000:
        raise SystemExit("flamegraph stack filter: quality ceiling must be 0..50000 ppm")

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
    if dropped_ppm > maximum_dropped_ppm:
        raise SystemExit(
            "flamegraph stack filter: unresolved call chains are "
            f"{dropped_ppm} ppm, above the {maximum_dropped_ppm} ppm limit"
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
                f"maximum_dropped_unresolved_ppm={maximum_dropped_ppm}",
            ]
        )
        + "\n",
        encoding="utf-8",
    )


if __name__ == "__main__":
    main()
