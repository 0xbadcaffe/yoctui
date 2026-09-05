#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

evidence_dir="artifacts/performance/results/low-overhead"

validate() {
  python3 - "$1" <<'PY'
from pathlib import Path
import hashlib
import json
import math
import statistics
import subprocess
import sys

root = Path(sys.argv[1])
suite = json.loads((root / "measurement.json").read_text(encoding="utf-8"))
if suite.get("schema") != "yoctui.performance.low-overhead-suite.v1":
    raise SystemExit("low-overhead suite schema is missing or unsupported")
revision = suite.get("revision")
if not isinstance(revision, str) or len(revision) != 40:
    raise SystemExit("low-overhead source revision must be exact")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
if suite.get("method") != {
    "release_profile": True,
    "clock": "CLOCK_MONOTONIC",
    "cpu_source": "/proc/PID/stat fields 14+15",
    "warmup_seconds": 10,
    "sample_window_seconds": 60,
    "sample_interval_seconds": 1,
    "statistic": "10_percent_trimmed_mean",
    "terminal": {"columns": 160, "rows": 50},
    "refresh_milliseconds": 100,
    "startup_excluded": True,
}:
    raise SystemExit("low-overhead measurement method changed")
thresholds = suite.get("thresholds")
if thresholds != {
    "idle_daemon_cpu_percent_one_logical_cpu": 0.20,
    "idle_client_cpu_percent_one_logical_cpu": 0.50,
    "combined_cpu_percent_one_logical_cpu": 1.00,
}:
    raise SystemExit("low-overhead thresholds changed")
if suite.get("startup", {}).get("maximum_excluded_startup_ms") != 5_000:
    raise SystemExit("startup exclusion is not explicitly bounded")
if suite.get("startup", {}).get("daemon_socket_ready_ms", 5_001) > 5_000:
    raise SystemExit("daemon startup exceeded its excluded bound")
if suite.get("startup", {}).get("client_first_frame_ms", 5_001) > 5_000:
    raise SystemExit("client startup exceeded its excluded bound")
if suite.get("pty_bytes_drained", 0) <= 0:
    raise SystemExit("attached client did not render into its real PTY")

for source, digest in suite.get("sources", {}).items():
    path = Path(source)
    if not path.is_file() or hashlib.sha256(path.read_bytes()).hexdigest() != digest:
        raise SystemExit(f"low-overhead source digest mismatch: {source}")
required_sources = {
    "crates/yoctui-cli/src/main.rs",
    "crates/yoctui-protocol/src/daemon_ipc.rs",
    "scripts/measure-low-overhead.py",
    "scripts/measure-process-overhead.py",
}
if set(suite.get("sources", {})) != required_sources:
    raise SystemExit("low-overhead source identity is incomplete")

expected = {
    "idle-daemon.json": ("optimized-idle-daemon", {"daemon"}),
    "idle-attached-client.json": ("optimized-idle-attached-client", {"daemon", "client"}),
}
binary_hash = suite.get("binary", {}).get("sha256")
if not isinstance(binary_hash, str) or len(binary_hash) != 64:
    raise SystemExit("low-overhead binary digest is absent")

def trimmed_mean(values):
    ordered = sorted(values)
    trim = len(ordered) // 10
    return statistics.fmean(ordered[trim:len(ordered) - trim])

records = {}
for filename, (scenario, roles) in expected.items():
    path = root / filename
    if hashlib.sha256(path.read_bytes()).hexdigest() != suite["artifacts"].get(filename):
        raise SystemExit(f"low-overhead artifact digest mismatch: {filename}")
    record = json.loads(path.read_text(encoding="utf-8"))
    records[filename] = record
    if record.get("schema") != "yoctui.performance.process-overhead.v1":
        raise SystemExit(f"{filename}: process-overhead schema changed")
    if record.get("scenario") != scenario or record.get("revision") != revision:
        raise SystemExit(f"{filename}: scenario or source identity changed")
    if record.get("binary", {}).get("sha256") != binary_hash:
        raise SystemExit(f"{filename}: binary identity changed")
    if set(record.get("processes", {})) != roles:
        raise SystemExit(f"{filename}: process roles changed")
    for identity in record["processes"].values():
        if not {"pid", "start_time_ticks_since_boot", "executable", "command"}.issubset(identity):
            raise SystemExit(f"{filename}: process identity is incomplete")
    measurement = record.get("measurement", {})
    if measurement.get("clock") != "CLOCK_MONOTONIC":
        raise SystemExit(f"{filename}: clock changed")
    if measurement.get("cpu_source") != "/proc/PID/stat fields 14+15":
        raise SystemExit(f"{filename}: CPU accounting source changed")
    if measurement.get("warmup_seconds") != 10 or measurement.get("sample_window_seconds") != 60:
        raise SystemExit(f"{filename}: steady-state window changed")
    if measurement.get("sample_count") != 60 or len(record.get("samples", [])) != 60:
        raise SystemExit(f"{filename}: expected exactly sixty raw samples")
    if measurement.get("statistic") != "10_percent_trimmed_mean":
        raise SystemExit(f"{filename}: robust statistic changed")
    if record.get("terminal") != {"columns": 160, "rows": 50, "refresh_milliseconds": 100}:
        raise SystemExit(f"{filename}: terminal configuration changed")
    host = record.get("host", {})
    if not {
        "kernel", "machine", "cpu_model", "logical_cpus", "online_cpus",
        "memory_total_bytes", "boot_id", "filesystem",
    }.issubset(host):
        raise SystemExit(f"{filename}: host identity is incomplete")
    samples = record["samples"]
    combined = [float(sample["combined_cpu_percent_one_logical_cpu"]) for sample in samples]
    observed_combined = float(record["summary"]["combined_cpu_trimmed_mean_percent_one_logical_cpu"])
    if not math.isclose(trimmed_mean(combined), observed_combined, rel_tol=1e-9, abs_tol=1e-9):
        raise SystemExit(f"{filename}: combined statistic cannot be reproduced")
    for role in roles:
        cpu = [float(sample["processes"][role]["cpu_percent_one_logical_cpu"]) for sample in samples]
        observed = float(record["summary"]["processes"][role]["cpu_trimmed_mean_percent_one_logical_cpu"])
        if not math.isclose(trimmed_mean(cpu), observed, rel_tol=1e-9, abs_tol=1e-9):
            raise SystemExit(f"{filename}: {role} statistic cannot be reproduced")

idle = records["idle-daemon.json"]["summary"]["processes"]["daemon"]
attached = records["idle-attached-client.json"]["summary"]
observed = {
    "idle_daemon_cpu_percent_one_logical_cpu": float(idle["cpu_trimmed_mean_percent_one_logical_cpu"]),
    "attached_daemon_cpu_percent_one_logical_cpu": float(attached["processes"]["daemon"]["cpu_trimmed_mean_percent_one_logical_cpu"]),
    "idle_client_cpu_percent_one_logical_cpu": float(attached["processes"]["client"]["cpu_trimmed_mean_percent_one_logical_cpu"]),
    "combined_cpu_percent_one_logical_cpu": float(attached["combined_cpu_trimmed_mean_percent_one_logical_cpu"]),
}
for key, value in observed.items():
    if not math.isclose(value, float(suite["observations"][key]), rel_tol=1e-9, abs_tol=1e-9):
        raise SystemExit(f"low-overhead suite observation mismatch: {key}")
for key, limit in thresholds.items():
    if observed[key] > limit:
        raise SystemExit(f"low-overhead threshold failed: {key}={observed[key]:.4f}% > {limit:.2f}%")
print(
    "low-overhead evidence valid: idle daemon "
    f"{observed['idle_daemon_cpu_percent_one_logical_cpu']:.4f}%, idle client "
    f"{observed['idle_client_cpu_percent_one_logical_cpu']:.4f}%, combined "
    f"{observed['combined_cpu_percent_one_logical_cpu']:.4f}% of one logical CPU"
)
PY
}

validate "$evidence_dir"
python3 -m unittest scripts/test_measure_low_overhead.py

binary="target/release/yoctui"
if [[ ! -x "$binary" ]]; then
  printf 'missing release executable; run cargo build --release -p yoctui --all-features\n' >&2
  exit 1
fi
current="$(mktemp -d /tmp/yoctui-low-overhead-gate.XXXXXX)"
cleanup() {
  find "$current" -type f -delete 2>/dev/null || true
  rmdir "$current" 2>/dev/null || true
}
trap cleanup EXIT
./scripts/measure-low-overhead.py \
  --binary "$binary" \
  --revision "$(git rev-parse HEAD)" \
  --output-directory "$current" >/dev/null
validate "$current"
printf 'current release low-overhead gate passed\n'
