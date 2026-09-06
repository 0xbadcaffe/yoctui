#!/usr/bin/env python3
"""Fail-closed Memcheck policy, including the explicitly approved fixed caches."""

from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path
import tomllib
import xml.etree.ElementTree as ET


# Reviewed in artifacts/performance/logger/memcheck-optimized.json. These are
# allocation identities, not a general allowance for upstream or small leaks.
CACHE_RULES = (
    (
        "logger_target_name",
        15,
        1,
        (
            "tui_logger::config::level_config::LevelConfig",
            "tui_logger::logger::inner::TuiLogger>::move_events",
        ),
    ),
    (
        "logger_target_map",
        148,
        1,
        (
            "tui_logger::config::level_config::LevelConfig",
            "tui_logger::logger::inner::TuiLogger>::move_events",
        ),
    ),
    (
        "logger_static_map",
        34832,
        1,
        ("tui_logger::logger::inner::TUI_LOGGER", "HashMap<u64, log::LevelFilter>"),
    ),
    ("timezone_local_types", 72, 1, ("jiff_core::tz::tzif::LocalTimeType",)),
    ("timezone_names", 120, 1, ("jiff_core::util::SmallStr<6>",)),
    ("timezone_arc", 280, 1, ("ArcInner<jiff_core::tz::tzif::MaybeNamedTimeZone>",)),
    ("timezone_transition_info", 300, 1, ("jiff_core::tz::tzif::TransitionInfo",)),
    ("timezone_timestamps", 1200, 1, ("jiff_core::tz::tzif::Timestamp",)),
    ("timezone_datetimes", 1200, 2, ("jiff_core::tz::tzif::DateTime",)),
)
DEPENDENCIES = {
    "tui-logger": (
        "0.18.3",
        "a6f73b6b6152df40e19f2ffc818c7259211955dd9b35072d361ddbc51c20bb61",
    ),
    "jiff": (
        "0.2.35",
        "668b7183bd07af9a4885f5c35b0cc5c83c4607a913c16b7e17291832910d2dcc",
    ),
    "jiff-core": (
        "0.1.0",
        "7feca88439efe53da3754500c1851dedf3cb36c524dd5cf8225cc0794de95d09",
    ),
}
CACHE_BYTE_LIMIT = 39367
CACHE_BLOCK_LIMIT = 10


def check_dependencies(lock_text: str) -> None:
    packages = tomllib.loads(lock_text).get("package", [])
    for name, identity in DEPENDENCIES.items():
        actual = [
            (p.get("version"), p.get("checksum"))
            for p in packages
            if p.get("name") == name
            and p.get("source")
            == "registry+https://github.com/rust-lang/crates.io-index"
        ]
        if actual != [identity] or sum(p.get("name") == name for p in packages) != 1:
            raise ValueError(f"cache exception requires dependency review: {name}")


def number(text: str | None) -> int:
    value = (text or "").replace(",", "")
    if not value.isascii() or not value.isdecimal():
        raise ValueError(f"missing or invalid Memcheck count: {text!r}")
    return int(value)


def classify_cache(error: ET.Element) -> str:
    size = number(error.findtext("xwhat/leakedbytes"))
    blocks = number(error.findtext("xwhat/leakedblocks"))
    stack = "\n".join(
        frame.findtext("fn", "") for frame in error.findall("stack/frame")
    )
    for name, expected_size, _, fragments in CACHE_RULES:
        required = fragments
        if name.startswith("timezone_"):
            required += (
                "jiff::tz::timezone::TimeZone",
                "jiff::tz::db::zoneinfo::inner::CachedTimeZone",
            )
        if (
            size == expected_size
            and blocks == 1
            and all(part in stack for part in required)
        ):
            return name
    raise ValueError(
        f"unreviewed possibly-lost allocation: {size} bytes / {blocks} blocks"
    )


def evaluate(xml: str) -> dict:
    # Valgrind 3.25+ occasionally emits this unescaped diagnostic token.
    root = ET.fromstring(xml.replace("<unknown>", "&lt;unknown&gt;"))
    states = root.findall("status/state")
    if (
        root.tag != "valgrindoutput"
        or root.findtext("protocoltool") != "memcheck"
        or not states
        or states[-1].text != "FINISHED"
        or root.find("fatal_signal") is not None
    ):
        raise ValueError("incomplete or non-Memcheck report")
    totals: Counter = Counter()
    caches: Counter = Counter()
    signal_fds = 0
    for error in root.findall("error"):
        kind = error.findtext("kind")
        if kind in ("Leak_PossiblyLost", "Leak_StillReachable"):
            size = number(error.findtext("xwhat/leakedbytes"))
            number(error.findtext("xwhat/leakedblocks"))
            totals[kind] += size
            if kind == "Leak_PossiblyLost":
                caches[classify_cache(error)] += 1
        elif kind == "FdNotClosed":
            if not any(
                "tokio::signal::" in frame.findtext("fn", "")
                for frame in error.findall("stack/frame")
            ):
                raise ValueError("unexpected open descriptor")
            signal_fds += 1
        else:
            raise ValueError(f"fatal Memcheck finding: {kind}")
    for name, _, maximum, _ in CACHE_RULES:
        if caches[name] > maximum:
            raise ValueError(f"fixed cache allocation count increased: {name}")
    if (
        totals["Leak_PossiblyLost"] > CACHE_BYTE_LIMIT
        or sum(caches.values()) > CACHE_BLOCK_LIMIT
    ):
        raise ValueError("fixed upstream cache ceiling exceeded")
    summary = root.find("leak_summary")
    if summary is not None:
        for category, expected in (
            ("definitely_lost", 0),
            ("indirectly_lost", 0),
            ("possibly_lost", totals["Leak_PossiblyLost"]),
            ("still_reachable", totals["Leak_StillReachable"]),
        ):
            if number(summary.findtext(f"{category}/bytes")) != expected:
                raise ValueError(f"leak summary mismatch: {category}")
    return {
        "schema": "yoctui.memcheck.bounded-cache.v1",
        "definitely_lost_bytes": 0,
        "indirectly_lost_bytes": 0,
        "accepted_cache_bytes": totals["Leak_PossiblyLost"],
        "accepted_cache_blocks": sum(caches.values()),
        "cache_allocations": dict(sorted(caches.items())),
        "still_reachable_bytes": totals["Leak_StillReachable"],
        "tokio_signal_descriptors": signal_fds,
    }


def check_growth(baseline: dict, current: dict) -> None:
    for name, _, _, _ in CACHE_RULES:
        if current["cache_allocations"].get(name, 0) > baseline[
            "cache_allocations"
        ].get(name, 0):
            raise ValueError(f"upstream cache grew with frame count: {name}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("report", type=Path)
    parser.add_argument("--baseline", type=Path)
    parser.add_argument("--lock", type=Path, default=Path("Cargo.lock"))
    args = parser.parse_args()
    check_dependencies(args.lock.read_text(encoding="utf-8"))
    result = evaluate(args.report.read_text(encoding="utf-8"))
    if args.baseline:
        baseline = evaluate(args.baseline.read_text(encoding="utf-8"))
        check_growth(baseline, result)
        result["no_cache_growth"] = True
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
