#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mode="${1:-all}"

verify_event_flood() {
  python3 -m unittest scripts/test_event_flood_harness.py
  cargo build -p yoctui >/dev/null
  artifact="$(mktemp /tmp/yoctui-event-flood-gate.XXXXXX.json)"
  trap 'unlink "$artifact" 2>/dev/null || true' RETURN
  ./scripts/event-flood-harness.py \
    --binary target/debug/yoctui \
    --rate 4000 \
    --duration-seconds 1 \
    --observation-seconds 1.5 \
    --expect-pre-backpressure-failure \
    --output "$artifact" >/dev/null
  python3 - "$artifact" <<'PY'
from pathlib import Path
import json
import sys

record = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
if record["schema"] != "yoctui.performance.event-flood-observation.v1":
    raise SystemExit("event flood observation schema mismatch")
generator = record["generator"]
if generator["configuration"]["rate_events_per_second"] < 2000:
    raise SystemExit("event flood did not request the contractual rate")
if generator["measurement"]["ordinary_events"] < 2000:
    raise SystemExit("event flood did not generate enough traffic")
counts = generator["measurement"]["event_counts"]
required = {
    "task_queued", "task_started", "task_progress", "task_completed",
    "log", "warning", "error", "build_completed",
}
if not required.issubset(counts):
    raise SystemExit("event flood mix is incomplete")
if not record["client"]["connection_continuity"]:
    raise SystemExit("observer client disconnected during the bounded fixture")
if not record["result"]["expected_pre_backpressure_terminal_starvation_observed"]:
    raise SystemExit("harness failed to expose the known pre-backpressure terminal starvation")
if record["result"]["critical_retention_passed"]:
    raise SystemExit("pre-backpressure behavior was incorrectly reported as passing")
if record["bounds"]["supervisor_ingress"] != "unbounded_pre_backpressure":
    raise SystemExit("pre-backpressure ingress was not identified honestly")
print("event flood harness valid: production path observed known terminal starvation")
PY
  trap - RETURN
  unlink "$artifact"
}

case "$mode" in
  --event-flood)
    verify_event_flood
    ;;
  all)
    printf '%s\n' 'strict IPC continuity gate is not implemented yet' >&2
    exit 1
    ;;
  *)
    printf 'unknown IPC continuity verification mode: %s\n' "$mode" >&2
    exit 2
    ;;
esac
