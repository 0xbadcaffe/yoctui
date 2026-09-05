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

verify_input_latency() {
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess

root = Path("artifacts/performance/input-latency")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.input-latency-manifest.v1":
    raise SystemExit("input-latency manifest schema is missing or unsupported")
revision = manifest.get("source_base_revision")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
artifact = root / manifest["artifact"]
if hashlib.sha256(artifact.read_bytes()).hexdigest() != manifest["artifact_sha256"]:
    raise SystemExit("input-latency evidence digest mismatch")
record = json.loads(artifact.read_text(encoding="utf-8"))
if record.get("schema") != "yoctui.performance.input-latency.v1":
    raise SystemExit("input-latency evidence schema is unsupported")
if record.get("revision") != revision:
    raise SystemExit("input-latency evidence source identity mismatch")
if record.get("binary", {}).get("sha256") != manifest["binary_sha256"]:
    raise SystemExit("input-latency binary identity mismatch")
method = manifest["method"]
if method != {
    "release_profile": True,
    "terminal_columns": 160,
    "terminal_rows": 50,
    "warmup_seconds": 1,
    "observations_per_path": 100,
    "clock": "CLOCK_MONOTONIC",
    "load": "one pinned worker per affinity CPU; no deliberately free CPU",
}:
    raise SystemExit("input-latency method no longer matches the reviewed contract")
if record["terminal"] != {"columns": 160, "rows": 50}:
    raise SystemExit("input-latency terminal dimensions mismatch")
if record["configuration"]["clock"] != "CLOCK_MONOTONIC":
    raise SystemExit("input-latency measurement did not use monotonic time")
for source, digest in manifest["sources"].items():
    if hashlib.sha256(Path(source).read_bytes()).hexdigest() != digest:
        raise SystemExit(f"input-latency source digest mismatch: {source}")
for kind in ("keyboard", "mouse"):
    samples = record["samples"].get(kind, [])
    if len(samples) != 100:
        raise SystemExit(f"input-latency path does not contain 100 samples: {kind}")
    for sequence, sample in enumerate(samples, 1):
        if sample["sequence"] != sequence:
            raise SystemExit(f"input-latency sample sequence mismatch: {kind}")
        if not sample["sent_ns"] <= sample["received_ns"] <= sample["model_ns"] <= sample["frame_ns"]:
            raise SystemExit(f"input-latency timestamp order mismatch: {kind}")
for metric in (
    "keyboard_to_model_ms",
    "keyboard_to_visible_frame_ms",
    "mouse_to_visible_selection_ms",
):
    summary = record["summary"][metric]
    if summary["p50"] < 0 or summary["p95"] < summary["p50"] or summary["p95"] > 100:
        raise SystemExit(f"input-latency threshold failed: {metric}")
load = record["saturation"]
affinity = record["host"]["affinity_cpus"]
if load["status"] != "completed" or load["cleanup"]["children_reaped"] is not True:
    raise SystemExit("input-latency saturation fixture did not clean up")
if load["configuration"]["selected_cpus"] != affinity:
    raise SystemExit("input-latency evidence deliberately left an affinity CPU free")
if load["configuration"]["requested_workers"] != len(affinity):
    raise SystemExit("input-latency evidence did not run one load worker per affinity CPU")
if load["achieved"]["minimum_worker_cpu_percent"] < 25:
    raise SystemExit("input-latency evidence did not sustain its declared load")
source = Path("crates/yoctui-e2e/examples/input_latency_probe.rs").read_text(encoding="utf-8")
for required in (
    "event::read()", "focus_action_for_app", "mouse_action_for_app", "update(&mut app",
    "yoctui_ui::render_at", "CLOCK_MONOTONIC",
):
    if required not in source:
        raise SystemExit(f"production input-latency probe path is missing: {required}")
summary = record["summary"]
print(
    "input latency valid under full-CPU load: model p95 "
    f"{summary['keyboard_to_model_ms']['p95']:.3f} ms, frame p95 "
    f"{summary['keyboard_to_visible_frame_ms']['p95']:.3f} ms, mouse p95 "
    f"{summary['mouse_to_visible_selection_ms']['p95']:.3f} ms"
)
PY

  python3 -m unittest scripts/test_measure_input_latency.py
  cargo build -q -p yoctui-e2e --example input_latency_probe
  current="$(mktemp /tmp/yoctui-input-latency-current.XXXXXX.json)"
  trap 'unlink "$current" 2>/dev/null || true' RETURN
  ./scripts/measure-input-latency.py \
    --binary target/debug/examples/input_latency_probe \
    --revision "$(git rev-parse HEAD)" \
    --warmup-seconds 0.5 \
    --observations 100 \
    --output "$current" >/dev/null
  python3 - "$current" <<'PY'
from pathlib import Path
import json
import sys

record = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
for metric in record["summary"].values():
    if metric["p95"] > 100:
        raise SystemExit("current input-latency regression exceeds 100 ms")
if record["saturation"]["achieved"]["minimum_worker_cpu_percent"] < 25:
    raise SystemExit("current input-latency regression lacked CPU saturation")
print("current keyboard, frame, and mouse latency paths remain below 100 ms")
PY
  trap - RETURN
  unlink "$current"
}

case "$mode" in
  --harness)
    verify_harness
    ;;
  --bitbake-connection)
    verify_bitbake_connection
    ;;
  --input-latency)
    verify_harness
    verify_input_latency
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
