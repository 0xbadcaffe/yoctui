#!/usr/bin/env python3
"""Bounded deterministic CPU saturation for Yoctui responsiveness tests."""

from __future__ import annotations

import argparse
import json
import multiprocessing as mp
import os
import signal
import sys
import time
from dataclasses import asdict, dataclass
from pathlib import Path


SCHEMA = "yoctui.performance.cpu-saturation.v1"
MASK = (1 << 64) - 1
CHUNK = 16_384


@dataclass(frozen=True)
class WorkerResult:
    worker: int
    pid: int
    cpu: int
    iterations: int
    checksum: int
    cpu_seconds: float
    elapsed_seconds: float
    completed: bool


def parse_cpu_list(value: str) -> list[int]:
    cpus: set[int] = set()
    try:
        for part in value.split(","):
            if "-" in part:
                first, last = (int(item) for item in part.split("-", 1))
                if first < 0 or last < first:
                    raise ValueError
                cpus.update(range(first, last + 1))
            else:
                cpu = int(part)
                if cpu < 0:
                    raise ValueError
                cpus.add(cpu)
    except ValueError as error:
        raise argparse.ArgumentTypeError("CPU list must use N or N-M ranges") from error
    if not cpus:
        raise argparse.ArgumentTypeError("CPU list must not be empty")
    return sorted(cpus)


def available_cpus() -> list[int]:
    if hasattr(os, "sched_getaffinity"):
        return sorted(os.sched_getaffinity(0))
    return list(range(os.cpu_count() or 1))


def proc_cpu() -> tuple[int, int]:
    fields = Path("/proc/stat").read_text(encoding="utf-8").splitlines()[0].split()[1:]
    values = [int(value) for value in fields]
    total = sum(values)
    idle = values[3] + (values[4] if len(values) > 4 else 0)
    return total, idle


def cpu_utilization(before: tuple[int, int], after: tuple[int, int]) -> float | None:
    total = after[0] - before[0]
    idle = after[1] - before[1]
    if total <= 0 or idle < 0 or idle > total:
        return None
    return (total - idle) * 100.0 / total


def burn_chunk(value: int) -> int:
    for _ in range(CHUNK):
        value ^= value << 13
        value ^= value >> 7
        value ^= value << 17
        value &= MASK
    return value


def worker_main(
    worker: int,
    cpu: int,
    warmup: float,
    duration: float,
    start: mp.synchronize.Event,
    stop: mp.synchronize.Event,
    connection: mp.connection.Connection,
) -> None:
    if hasattr(os, "sched_setaffinity"):
        os.sched_setaffinity(0, {cpu})
    connection.send({"type": "ready", "worker": worker, "pid": os.getpid(), "cpu": cpu})
    if not start.wait(timeout=10):
        connection.send({"type": "error", "worker": worker, "message": "start timeout"})
        return
    value = (0x9E3779B97F4A7C15 ^ (worker + 1)) & MASK
    warmup_deadline = time.monotonic() + warmup
    while time.monotonic() < warmup_deadline and not stop.is_set():
        value = burn_chunk(value)
    iterations = 0
    cpu_started = time.process_time()
    started = time.monotonic()
    deadline = started + duration
    while time.monotonic() < deadline and not stop.is_set():
        value = burn_chunk(value)
        iterations += CHUNK
    elapsed = time.monotonic() - started
    connection.send(
        {
            "type": "result",
            **asdict(
                WorkerResult(
                    worker=worker,
                    pid=os.getpid(),
                    cpu=cpu,
                    iterations=iterations,
                    checksum=value,
                    cpu_seconds=time.process_time() - cpu_started,
                    elapsed_seconds=elapsed,
                    completed=not stop.is_set() and elapsed >= duration,
                )
            ),
        }
    )


def event(stream, kind: str, started: float, **fields: object) -> None:
    if stream is None:
        return
    stream.write(
        json.dumps(
            {
                "schema": SCHEMA,
                "event": kind,
                "elapsed_seconds": time.monotonic() - started,
                **fields,
            },
            separators=(",", ":"),
        )
        + "\n"
    )
    stream.flush()


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--workers", type=int)
    parser.add_argument("--cpu-list", type=parse_cpu_list)
    parser.add_argument("--warmup-seconds", type=float, default=1.0)
    parser.add_argument("--duration-seconds", type=float, default=5.0)
    parser.add_argument("--readiness-timeout-seconds", type=float, default=5.0)
    parser.add_argument("--minimum-worker-cpu-percent", type=float, default=75.0)
    parser.add_argument("--output", type=Path)
    parser.add_argument("--event-log", type=Path)
    args = parser.parse_args()
    allowed = available_cpus()
    selected = args.cpu_list or allowed
    if any(cpu not in allowed for cpu in selected):
        parser.error("CPU list contains a CPU outside the process affinity set")
    worker_count = args.workers if args.workers is not None else len(selected)
    if worker_count <= 0 or worker_count > len(selected):
        parser.error("workers must be within 1..selected CPU count")
    if args.warmup_seconds < 0 or args.duration_seconds < 0.25:
        parser.error(
            "warmup must be nonnegative and duration must be at least 0.25 seconds"
        )
    if args.readiness_timeout_seconds <= 0:
        parser.error("readiness timeout must be positive")
    if not 0 <= args.minimum_worker_cpu_percent <= 100:
        parser.error("minimum worker CPU percent must be within 0..100")
    selected = selected[:worker_count]

    context = mp.get_context("spawn")
    start = context.Event()
    stop = context.Event()
    processes: list[mp.Process] = []
    readers = []
    begun = time.monotonic()
    events = args.event_log.open("w", encoding="utf-8") if args.event_log else None
    interrupted = False

    def request_stop(_signum: int, _frame: object) -> None:
        nonlocal interrupted
        interrupted = True
        stop.set()

    previous_term = signal.signal(signal.SIGTERM, request_stop)
    previous_int = signal.signal(signal.SIGINT, request_stop)
    ready = []
    results = []
    before_cpu = proc_cpu()
    load_before = list(os.getloadavg())
    try:
        event(events, "starting", begun, workers=worker_count, cpus=selected)
        for worker, cpu in enumerate(selected):
            reader, writer = context.Pipe(duplex=False)
            process = context.Process(
                target=worker_main,
                args=(
                    worker,
                    cpu,
                    args.warmup_seconds,
                    args.duration_seconds,
                    start,
                    stop,
                    writer,
                ),
                name=f"yoctui-cpu-load-{worker}",
            )
            process.start()
            writer.close()
            processes.append(process)
            readers.append(reader)
        readiness_deadline = time.monotonic() + args.readiness_timeout_seconds
        for reader in readers:
            remaining = readiness_deadline - time.monotonic()
            if remaining <= 0 or not reader.poll(remaining):
                raise RuntimeError("CPU saturation worker readiness timed out")
            message = reader.recv()
            if message.get("type") != "ready":
                raise RuntimeError(
                    f"CPU saturation worker failed before readiness: {message}"
                )
            ready.append(message)
            event(events, "worker_ready", begun, **message)
        measurement_started = time.monotonic() + args.warmup_seconds
        event(
            events,
            "ready",
            begun,
            workers=len(ready),
            measurement_starts_in=args.warmup_seconds,
        )
        start.set()
        result_deadline = measurement_started + args.duration_seconds + 2.0
        for reader in readers:
            while not interrupted:
                remaining = result_deadline - time.monotonic()
                if remaining <= 0 or not reader.poll(min(0.1, remaining)):
                    if remaining <= 0:
                        raise RuntimeError(
                            "CPU saturation worker exceeded its hard deadline"
                        )
                    continue
                message = reader.recv()
                if message.get("type") != "result":
                    raise RuntimeError(f"CPU saturation worker failed: {message}")
                results.append(message)
                event(events, "worker_complete", begun, **message)
                break
        status = "interrupted" if interrupted else "completed"
    except BaseException:
        stop.set()
        status = "failed"
        raise
    finally:
        stop.set()
        for process in processes:
            process.join(timeout=1)
        for process in processes:
            if process.is_alive():
                process.terminate()
        for process in processes:
            process.join(timeout=1)
        for process in processes:
            if process.is_alive():
                process.kill()
                process.join()
        signal.signal(signal.SIGTERM, previous_term)
        signal.signal(signal.SIGINT, previous_int)

    after_cpu = proc_cpu()
    total_cpu_seconds = sum(float(result["cpu_seconds"]) for result in results)
    worker_percentages = [
        min(
            100.0,
            float(result["cpu_seconds"]) / float(result["elapsed_seconds"]) * 100.0,
        )
        for result in results
        if float(result["elapsed_seconds"]) > 0
    ]
    record = {
        "schema": SCHEMA,
        "status": status,
        "configuration": {
            "requested_workers": worker_count,
            "available_affinity_cpus": allowed,
            "selected_cpus": selected,
            "default_saturates_full_affinity": args.workers is None
            and args.cpu_list is None,
            "warmup_seconds": args.warmup_seconds,
            "duration_seconds": args.duration_seconds,
            "readiness_timeout_seconds": args.readiness_timeout_seconds,
        },
        "timing": {
            "clock": "CLOCK_MONOTONIC",
            "total_elapsed_seconds": time.monotonic() - begun,
        },
        "readiness": ready,
        "workers": results,
        "achieved": {
            "total_worker_cpu_seconds": total_cpu_seconds,
            "aggregate_worker_cpu_percent_one_logical_cpu": min(
                worker_count * 100.0,
                total_cpu_seconds / args.duration_seconds * 100.0,
            ),
            "minimum_worker_cpu_percent": min(worker_percentages, default=0.0),
            "mean_worker_cpu_percent": sum(worker_percentages) / len(worker_percentages)
            if worker_percentages
            else 0.0,
            "host_cpu_utilization_percent": cpu_utilization(before_cpu, after_cpu),
            "load_average_before": load_before,
            "load_average_after": list(os.getloadavg()),
        },
        "cleanup": {
            "children_reaped": all(not process.is_alive() for process in processes),
            "worker_pids": [process.pid for process in processes],
        },
    }
    event(events, status, begun, children_reaped=record["cleanup"]["children_reaped"])
    if events:
        events.close()
    rendered = json.dumps(record, indent=2) + "\n"
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(rendered, encoding="utf-8")
    sys.stdout.write(rendered)
    if interrupted:
        return 130
    if len(results) != worker_count or not record["cleanup"]["children_reaped"]:
        return 1
    if min(worker_percentages, default=0.0) < args.minimum_worker_cpu_percent:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
