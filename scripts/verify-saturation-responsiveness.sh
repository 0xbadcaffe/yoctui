#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mode="${1:-all}"

verify_harness() {
  python3 -m unittest scripts/test_cpu_saturation_harness.py
  artifact="$(mktemp /tmp/yoctui-saturation-gate.XXXXXX.json)"
  trap 'unlink "$artifact" 2>/dev/null || true' RETURN
  ./scripts/cpu-saturation-harness.py \
    --warmup-seconds 0.25 \
    --duration-seconds 1 \
    --minimum-worker-cpu-percent 60 \
    --output "$artifact" >/dev/null
  python3 - "$artifact" <<'PY'
from pathlib import Path
import json
import os
import sys

record = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
configuration = record["configuration"]
available = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else os.cpu_count()
if configuration["requested_workers"] != available:
    raise SystemExit("default saturation fixture did not use the full affinity set")
if configuration["selected_cpus"] != configuration["available_affinity_cpus"]:
    raise SystemExit("default saturation fixture deliberately left a CPU free")
if len(record["readiness"]) != available or len(record["workers"]) != available:
    raise SystemExit("not every saturation worker became ready and completed")
if record["status"] != "completed" or not record["cleanup"]["children_reaped"]:
    raise SystemExit("saturation fixture did not exit cleanly")
if record["achieved"]["minimum_worker_cpu_percent"] < 60:
    raise SystemExit("saturation fixture did not achieve the declared worker load")
print(f"CPU saturation harness valid: {available} affinity CPUs, no reserved core")
PY
  trap - RETURN
  unlink "$artifact"
}

case "$mode" in
  --harness)
    verify_harness
    ;;
  all)
    printf '%s\n' 'full saturation responsiveness gate is not implemented yet' >&2
    exit 1
    ;;
  *)
    printf 'unknown saturation verification mode: %s\n' "$mode" >&2
    exit 2
    ;;
esac
