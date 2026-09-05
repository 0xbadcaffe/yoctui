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

case "$mode" in
  --contract)
    verify_contract
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
