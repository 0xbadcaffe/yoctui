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

verify_bitbake_connection() {
  python3 - <<'PY'
from pathlib import Path

supervisor = Path("crates/yoctui-cli/src/daemon_bitbake.rs").read_text(encoding="utf-8")
backend = Path("crates/yoctui-bitbake/src/lib.rs").read_text(encoding="utf-8")
bridge = Path("crates/yoctui-bitbake/bridge/yoctui_bridge.py").read_text(encoding="utf-8")

next_event = backend.split(
    "async fn next_event(&mut self) -> Result<BackendEvent, BackendError> {", 2
)[-1].split("async fn shutdown", 1)[0]
if "tokio::time::timeout" in next_event or "sleep" in next_event:
    raise SystemExit("BitBake event liveness regressed to elapsed-time polling")
if "return Ok(BackendEvent::Disconnected);" not in next_event:
    raise SystemExit("real bridge EOF no longer maps to a typed disconnect")
for required in (
    "tokio::time::Instant::now() + cancellation_terminal_timeout",
    "biased;",
    "Terminal publication is a correctness boundary",
):
    if required not in supervisor:
        raise SystemExit(f"saturation-tolerant supervisor contract is missing: {required}")
terminal_send = supervisor.index("cancellation_terminal_tx.send(DaemonBitBakeEvent::Backend")
cleanup = supervisor.index("backend.terminate_server()", terminal_send)
if terminal_send >= cleanup:
    raise SystemExit("cancellation terminal is gated by post-terminal cleanup")
for required in (
    "MAX_NATIVE_EVENTS_PER_POLL = 64",
    "selector.select(0.1 if adapter.build_active else 1.0)",
    "wait_event(0.01 if first else 0)",
):
    if required not in bridge:
        raise SystemExit(f"bounded native-event scheduling contract is missing: {required}")
print("BitBake saturation source contracts valid")
PY

  cargo test -q -p yoctui --bin yoctui bitbake_connection_ --no-run
  cargo test -q -p yoctui --bin yoctui \
    daemon_compatibility_cancellation_preempts_event_flood_and_terminates_once --no-run

  artifact="$(mktemp /tmp/yoctui-bitbake-saturation.XXXXXX.json)"
  event_log="$(mktemp /tmp/yoctui-bitbake-saturation.XXXXXX.jsonl)"
  ./scripts/cpu-saturation-harness.py \
    --warmup-seconds 0.25 \
    --duration-seconds 4 \
    --minimum-worker-cpu-percent 35 \
    --event-log "$event_log" \
    --output "$artifact" >/dev/null &
  load_pid="$!"
  cleanup_bitbake_fixture() {
    if kill -0 "$load_pid" 2>/dev/null; then
      kill "$load_pid" 2>/dev/null || true
      wait "$load_pid" 2>/dev/null || true
    fi
    unlink "$artifact" 2>/dev/null || true
    unlink "$event_log" 2>/dev/null || true
  }
  trap cleanup_bitbake_fixture RETURN

  ready=false
  for _ in $(seq 1 300); do
    if rg -q '"event":"ready"' "$event_log"; then
      ready=true
      break
    fi
    sleep 0.02
  done
  if [[ "$ready" != true ]]; then
    printf '%s\n' 'CPU saturation fixture did not become ready' >&2
    return 1
  fi

  cargo test -q -p yoctui --bin yoctui bitbake_connection_
  cargo test -q -p yoctui --bin yoctui \
    daemon_compatibility_cancellation_preempts_event_flood_and_terminates_once
  wait "$load_pid"

  python3 - "$artifact" <<'PY'
from pathlib import Path
import json
import os
import sys

record = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
available = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else os.cpu_count()
if record["status"] != "completed" or not record["cleanup"]["children_reaped"]:
    raise SystemExit("BitBake connection load fixture did not complete cleanly")
if len(record["workers"]) != available:
    raise SystemExit("BitBake connection gate did not keep every available CPU runnable")
if record["achieved"]["minimum_worker_cpu_percent"] < 35:
    raise SystemExit("BitBake connection gate did not sustain CPU pressure")
print(
    "BitBake connection remains correct under full-affinity saturation: "
    "delayed events survived, real EOF reported, cancellation acknowledged"
)
PY
  trap - RETURN
  unlink "$artifact"
  unlink "$event_log"
}

case "$mode" in
  --harness)
    verify_harness
    ;;
  --bitbake-connection)
    verify_bitbake_connection
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
