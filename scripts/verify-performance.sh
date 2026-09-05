#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mode="${1:-all}"

verify_contract() {
  python3 - <<'PY'
from pathlib import Path
import tomllib

contract = " ".join(Path("docs/performance.md").read_text(encoding="utf-8").split())
required = [
    "one percent of one logical CPU",
    "at most 1.00% of one logical CPU",
    "10-second warmup",
    "60 one-second samples",
    "fixed 160x50 PTY",
    "Idle daemon",
    "Idle attached client",
    "Active build",
    "PTY attached but idle",
    "High-rate BitBake stream",
    "Two attached clients",
    "key press to visible frame p95 <=100 ms",
    "daemon event to client receipt p50 <=25 ms and p95 <=100 ms",
    "three consecutive replies are absent over at least 90 seconds",
    "daemon RSS growth after warmup <=32 MiB",
    "client RSS growth after warmup <=32 MiB",
    "artifacts/performance/baseline/",
    "Real-Poky evidence",
]
missing = [text for text in required if text not in contract]
if missing:
    raise SystemExit("performance contract is incomplete: " + ", ".join(missing))

data = tomllib.loads(Path("docs/task-registry.toml").read_text(encoding="utf-8"))
tasks = {task["id"]: task for task in data["task"]}
required_ids = {
    "PERF-SPEC-001", "PERF-BASELINE-001", "PERF-FLAMEGRAPH-001",
    "PERF-WAKEUPS-001", "PERF-EVENTLOOP-001", "PERF-RENDER-001",
    "PERF-ANIM-001", "PERF-TELEMETRY-001", "PERF-LOG-001",
    "PERF-TASKS-001", "PERF-IPC-001", "PERF-IPC-BACKPRESSURE-001",
    "PERF-BITBAKE-CONN-001", "PERF-TOKIO-001", "PERF-SCHED-001",
    "PERF-CPU-AFFINITY-001", "PERF-BITBAKE-COEXIST-001",
    "PERF-INPUT-LATENCY-001", "PERF-IPC-LATENCY-001",
    "PERF-SATURATION-HARNESS-001", "PERF-EVENT-FLOOD-001",
    "PERF-REAL-POKY-001", "PERF-CPU-GATE-001",
    "PERF-RESPONSIVENESS-GATE-001", "PERF-IPC-GATE-001",
    "PERF-MEMORY-GATE-001", "PERF-REGRESSION-001", "PERF-CI-001",
    "PERF-DOC-001", "PERF-001",
}
missing_ids = sorted(required_ids - tasks.keys())
if missing_ids:
    raise SystemExit("missing M46 task(s): " + ", ".join(missing_ids))
for task_id in required_ids:
    task = tasks[task_id]
    if task.get("milestone") != "M46" or task.get("required") is not True:
        raise SystemExit(f"{task_id}: must be a required M46 task")
print(f"performance contract valid: {len(required_ids)} required M46 tasks")
PY
}

verify_all_done() {
  python3 - <<'PY'
from pathlib import Path
import tomllib

tasks = tomllib.loads(Path("docs/task-registry.toml").read_text(encoding="utf-8"))["task"]
incomplete = [task["id"] for task in tasks if task.get("required") and task["id"].startswith("PERF-") and task["status"] != "DONE"]
if incomplete:
    raise SystemExit("required performance tasks are incomplete: " + ", ".join(incomplete))
print("all required performance tasks are DONE")
PY
}

verify_baseline() {
  python3 - <<'PY'
from pathlib import Path
import ast
import hashlib
import json
import subprocess

root = Path("artifacts/performance/baseline")
manifest_path = root / "manifest.json"
manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.baseline-manifest.v1":
    raise SystemExit("baseline manifest schema is missing or unsupported")
revision = manifest.get("revision")
if not isinstance(revision, str) or len(revision) != 40:
    raise SystemExit("baseline revision must be an exact commit")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
)
if manifest.get("terminal") != {
    "columns": 160,
    "rows": 50,
    "refresh_milliseconds": 100,
}:
    raise SystemExit("baseline terminal configuration is not the contract configuration")

expected = {
    "active-real-poky-build.json": "active-real-poky-build",
    "event-flood.json": "bitbake-event-flood-2000-per-second",
    "idle-attached-client.json": "idle-attached-client",
    "idle-daemon.json": "idle-daemon",
    "pty-attached-active.json": "pty-attached-active-100-lines-per-second",
    "pty-attached-idle.json": "pty-attached-idle",
    "two-attached-clients.json": "two-attached-clients",
}
declared = manifest.get("artifacts")
if not isinstance(declared, dict) or set(declared) != set(expected):
    raise SystemExit("baseline manifest does not contain the exact scenario set")

binary_hash = manifest.get("binary_sha256")
for filename, scenario in expected.items():
    path = root / filename
    payload = path.read_bytes()
    if hashlib.sha256(payload).hexdigest() != declared[filename]:
        raise SystemExit(f"baseline artifact digest mismatch: {filename}")
    record = json.loads(payload)
    if record.get("schema") != "yoctui.performance.process-overhead.v1":
        raise SystemExit(f"{filename}: unsupported process-overhead schema")
    if record.get("scenario") != scenario:
        raise SystemExit(f"{filename}: scenario identity mismatch")
    if record.get("revision") != revision:
        raise SystemExit(f"{filename}: mixed source revisions are forbidden")
    if record.get("binary", {}).get("sha256") != binary_hash:
        raise SystemExit(f"{filename}: mixed binary hashes are forbidden")
    measurement = record.get("measurement", {})
    if measurement.get("warmup_seconds") != 10:
        raise SystemExit(f"{filename}: warmup is not 10 seconds")
    if measurement.get("sample_window_seconds") != 60:
        raise SystemExit(f"{filename}: window is not 60 seconds")
    if measurement.get("sample_count") != 60:
        raise SystemExit(f"{filename}: expected sixty raw samples")
    if measurement.get("statistic") != "10_percent_trimmed_mean":
        raise SystemExit(f"{filename}: robust statistic is absent")
    if record.get("terminal") != manifest["terminal"]:
        raise SystemExit(f"{filename}: terminal configuration mismatch")
    host = record.get("host", {})
    required_host = {
        "kernel", "machine", "cpu_model", "logical_cpus", "online_cpus",
        "memory_total_bytes", "boot_id", "filesystem",
    }
    if not required_host.issubset(host):
        raise SystemExit(f"{filename}: host identity is incomplete")
    if not record.get("samples") or not record.get("summary", {}).get("processes"):
        raise SystemExit(f"{filename}: raw samples or process summary are absent")

observations = manifest.get("observations", {})
required_observations = {
    "idle_daemon_cpu_percent", "idle_attached_combined_cpu_percent",
    "active_real_poky_combined_cpu_percent", "pty_idle_combined_cpu_percent",
    "pty_active_combined_cpu_percent", "event_flood_combined_cpu_percent",
    "two_client_combined_cpu_percent", "idle_render_invocations_per_second",
    "idle_daemon_voluntary_context_switches_per_second",
    "flood_terminal_outcome_starved",
}
if not required_observations.issubset(observations):
    raise SystemExit("baseline observations are incomplete")
if observations["flood_terminal_outcome_starved"] is not True:
    raise SystemExit("baseline must retain the observed flood terminal starvation")

for source in (
    Path("scripts/measure-process-overhead.py"),
    Path("scripts/fixtures/bitbake-event-flood-bridge.py"),
):
    ast.parse(source.read_text(encoding="utf-8"), filename=str(source))
print(f"performance baseline valid: {len(expected)} scenarios at {revision[:12]}")
PY
}

case "$mode" in
  --contract)
    verify_contract
    ;;
  --baseline)
    verify_contract
    verify_baseline
    ;;
  all)
    verify_contract
    verify_all_done
    for gate in \
      ./scripts/verify-low-overhead.sh \
      ./scripts/verify-saturation-responsiveness.sh \
      ./scripts/verify-ipc-continuity.sh \
      ./scripts/verify-bounded-memory.sh
    do
      if [[ ! -x "$gate" ]]; then
        printf 'missing executable performance gate: %s\n' "$gate" >&2
        exit 1
      fi
      "$gate"
    done
    ;;
  *)
    printf 'performance verification mode is not implemented yet: %s\n' "$mode" >&2
    exit 1
    ;;
esac
