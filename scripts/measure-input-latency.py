#!/usr/bin/env python3
"""Measure production input/reducer/render latency through a real PTY."""

from __future__ import annotations

import argparse
import fcntl
import hashlib
import json
import math
import os
from pathlib import Path
import pty
import re
import select
import shutil
import statistics
import struct
import subprocess
import tempfile
import termios
import time


SCHEMA = "yoctui.performance.input-latency.v1"
READY = re.compile(rb"\x1b\]777;yoctui-input-latency;ready;(\d+)\x07")
MARKER = re.compile(
    rb"\x1b\]777;yoctui-input-latency;(keyboard|mouse);(\d+);"
    rb"(\d+);(\d+);(\d+);(\d+)\x07"
)


def percentile(values: list[float], fraction: float) -> float:
    if not values:
        raise ValueError("percentile requires a sample")
    ordered = sorted(values)
    return ordered[max(0, math.ceil(fraction * len(ordered)) - 1)]


def summary(values: list[float]) -> dict[str, float]:
    return {
        "p50": percentile(values, 0.50),
        "p95": percentile(values, 0.95),
        "maximum": max(values),
        "mean": statistics.fmean(values),
    }


def wait_marker(
    master: int,
    buffered: bytearray,
    pattern: re.Pattern[bytes],
    timeout: float,
) -> re.Match[bytes]:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        match = pattern.search(buffered)
        if match:
            matched = bytes(match.group(0))
            del buffered[: match.end()]
            stable = pattern.fullmatch(matched)
            assert stable is not None
            return stable
        readable, _, _ = select.select([master], [], [], min(0.1, deadline - time.monotonic()))
        if not readable:
            continue
        try:
            chunk = os.read(master, 65_536)
        except BlockingIOError:
            continue
        if not chunk:
            break
        buffered.extend(chunk)
        if len(buffered) > 2 * 1024 * 1024:
            del buffered[: len(buffered) - 1024 * 1024]
    raise RuntimeError("timed out waiting for input-latency probe marker")


def wait_saturation_ready(process: subprocess.Popen[str], event_log: Path) -> None:
    deadline = time.monotonic() + 10.0
    while process.poll() is None and time.monotonic() < deadline:
        if event_log.exists() and '"event":"ready"' in event_log.read_text(
            encoding="utf-8"
        ):
            return
        time.sleep(0.02)
    raise RuntimeError("CPU saturation fixture did not become ready")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--warmup-seconds", type=float, default=1.0)
    parser.add_argument("--observations", type=int, default=100)
    args = parser.parse_args()
    if len(args.revision) != 40:
        parser.error("revision must be a full 40-character Git commit")
    if args.warmup_seconds < 0.25:
        parser.error("warmup must be at least 0.25 seconds")
    if args.observations < 100 or args.observations > 1_000:
        parser.error("observations must be within 100..1000")
    binary = args.binary.resolve()
    if not binary.is_file():
        parser.error("probe binary does not exist")

    root = Path(__file__).resolve().parents[1]
    fixture = Path(tempfile.mkdtemp(prefix="yoctui-input-latency-"))
    saturation_path = fixture / "saturation.json"
    saturation_events = fixture / "saturation.jsonl"
    master, slave = pty.openpty()
    fcntl.ioctl(slave, termios.TIOCSWINSZ, struct.pack("HHHH", 50, 160, 0, 0))
    probe = subprocess.Popen(
        [str(binary)],
        cwd=root,
        stdin=slave,
        stdout=slave,
        stderr=slave,
        start_new_session=True,
        close_fds=True,
    )
    os.close(slave)
    os.set_blocking(master, False)
    buffered = bytearray()
    saturation = None
    samples = {"keyboard": [], "mouse": []}
    try:
        ready = wait_marker(master, buffered, READY, 10.0)
        supported = int(ready.group(1))
        if args.observations > supported:
            raise RuntimeError(f"probe supports only {supported} observations per input kind")
        saturation = subprocess.Popen(
            [
                str(root / "scripts/cpu-saturation-harness.py"),
                "--warmup-seconds",
                str(args.warmup_seconds),
                "--duration-seconds",
                "20",
                "--minimum-worker-cpu-percent",
                "25",
                "--event-log",
                str(saturation_events),
                "--output",
                str(saturation_path),
            ],
            cwd=root,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.PIPE,
            text=True,
        )
        wait_saturation_ready(saturation, saturation_events)
        time.sleep(args.warmup_seconds + 0.1)

        previous_selection = None
        inputs = {
            "keyboard": (b"\x1b[B", b"\x1b[A"),
            "mouse": (b"\x1b[<65;5;5M", b"\x1b[<64;5;5M"),
        }
        for kind in ("keyboard", "mouse"):
            for index in range(1, args.observations + 1):
                payload = inputs[kind][(index - 1) % 2]
                sent_ns = time.clock_gettime_ns(time.CLOCK_MONOTONIC)
                os.write(master, payload)
                marker = wait_marker(master, buffered, MARKER, 2.0)
                observed_kind = marker.group(1).decode()
                sequence = int(marker.group(2))
                received_ns = int(marker.group(3))
                model_ns = int(marker.group(4))
                frame_ns = int(marker.group(5))
                selection = int(marker.group(6))
                if observed_kind != kind or sequence != index:
                    raise RuntimeError("probe marker order or input identity changed")
                if not sent_ns <= received_ns <= model_ns <= frame_ns:
                    raise RuntimeError("monotonic input timestamps are inconsistent")
                if previous_selection == selection:
                    raise RuntimeError(
                        f"{kind} input {index} did not change visible selection {selection}"
                    )
                previous_selection = selection
                samples[kind].append(
                    {
                        "sequence": sequence,
                        "sent_ns": sent_ns,
                        "received_ns": received_ns,
                        "model_ns": model_ns,
                        "frame_ns": frame_ns,
                        "selection": selection,
                        "delivery_latency_ms": (received_ns - sent_ns) / 1_000_000,
                        "model_latency_ms": (model_ns - sent_ns) / 1_000_000,
                        "frame_latency_ms": (frame_ns - sent_ns) / 1_000_000,
                    }
                )

        probe.wait(timeout=5.0)
        saturation_stderr = saturation.communicate(timeout=25.0)[1]
        if saturation.returncode != 0:
            raise RuntimeError(f"CPU saturation fixture failed: {saturation_stderr.strip()}")
        load = json.loads(saturation_path.read_text(encoding="utf-8"))
        keyboard_model = [sample["model_latency_ms"] for sample in samples["keyboard"]]
        keyboard_frame = [sample["frame_latency_ms"] for sample in samples["keyboard"]]
        mouse_frame = [sample["frame_latency_ms"] for sample in samples["mouse"]]
        record = {
            "schema": SCHEMA,
            "revision": args.revision,
            "captured_at_unix_ms": int(time.time() * 1_000),
            "binary": {
                "path": str(binary),
                "sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
            },
            "host": {
                "logical_cpus": os.cpu_count(),
                "affinity_cpus": sorted(os.sched_getaffinity(0)),
                "kernel": os.uname().release,
            },
            "terminal": {"columns": 160, "rows": 50},
            "configuration": {
                "clock": "CLOCK_MONOTONIC",
                "warmup_seconds": args.warmup_seconds,
                "observations_per_path": args.observations,
                "sequential_input": True,
                "load": "one pinned worker per affinity CPU; no deliberately free CPU",
            },
            "summary": {
                "keyboard_to_model_ms": summary(keyboard_model),
                "keyboard_to_visible_frame_ms": summary(keyboard_frame),
                "mouse_to_visible_selection_ms": summary(mouse_frame),
            },
            "samples": samples,
            "saturation": {
                "status": load["status"],
                "configuration": load["configuration"],
                "achieved": load["achieved"],
                "cleanup": load["cleanup"],
            },
        }
        rendered = json.dumps(record, indent=2, sort_keys=True) + "\n"
        args.output.parent.mkdir(parents=True, exist_ok=True)
        temporary = args.output.with_suffix(args.output.suffix + ".tmp")
        temporary.write_text(rendered, encoding="utf-8")
        temporary.replace(args.output)
        print(rendered, end="")
        return 0
    finally:
        os.close(master)
        if probe.poll() is None:
            probe.terminate()
            try:
                probe.wait(timeout=1.0)
            except subprocess.TimeoutExpired:
                probe.kill()
                probe.wait(timeout=1.0)
        if saturation is not None and saturation.poll() is None:
            saturation.terminate()
            try:
                saturation.wait(timeout=1.0)
            except subprocess.TimeoutExpired:
                saturation.kill()
                saturation.wait(timeout=1.0)
        shutil.rmtree(fixture)


if __name__ == "__main__":
    raise SystemExit(main())
