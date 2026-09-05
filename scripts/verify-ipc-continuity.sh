#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mode="${1:-all}"

verify_backpressure() {
  python3 -m unittest scripts/test_event_flood_harness.py
  cargo build -p yoctui >/dev/null
  artifact="$(mktemp /tmp/yoctui-event-flood-gate.XXXXXX.json)"
  trap 'unlink "$artifact" 2>/dev/null || true' RETURN
  ./scripts/event-flood-harness.py \
    --binary target/debug/yoctui \
    --rate 4000 \
    --duration-seconds 1 \
    --observation-seconds 3 \
    --include-slow-client \
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
if not record["client"]["reconnect_probe_succeeded"]:
    raise SystemExit("post-flood reconnect probe failed")
if not record["result"]["critical_retention_passed"]:
    raise SystemExit("bounded backpressure lost a critical sentinel")
if record["client"]["critical_missing"]:
    raise SystemExit("bounded backpressure omitted critical records")
if record["client"]["snapshot_replacements"] or record["client"]["resync_requests"]:
    raise SystemExit("healthy client required replacement under pressure")
if not record["client"]["event_sequences_strictly_increasing"]:
    raise SystemExit("critical/incremental event ordering regressed")
bounds = record["bounds"]
if bounds["supervisor_ingress"] != "bounded_priority_lanes":
    raise SystemExit("BitBake supervisor ingress is not bounded")
if bounds["supervisor_reliable_events"] != 512 or bounds["supervisor_cosmetic_events"] != 512:
    raise SystemExit("BitBake priority lane capacities changed without contract review")
if bounds["per_client_backlog_events"] != 4096:
    raise SystemExit("per-client cursor backlog is not explicitly bounded")
pressure = record["client"]["pressure"]
required_pressure = {
    "current_queue_depth", "maximum_queue_depth", "cosmetic_coalesced",
    "cosmetic_dropped", "reliable_waits", "forced_resynchronizations",
    "slow_client_disconnects",
}
if not required_pressure.issubset(pressure):
    raise SystemExit("daemon pressure counters are incomplete")
if pressure["maximum_queue_depth"] <= 0:
    raise SystemExit("daemon did not expose a queue high-water mark")
if pressure["slow_client_disconnects"] < 1:
    raise SystemExit("non-reading client was not isolated")
print("IPC backpressure valid: critical retention, slow-client isolation, and reconnect passed")
PY
  trap - RETURN
  unlink "$artifact"
}

verify_source_and_unit_contracts() {
  python3 - <<'PY'
from pathlib import Path

supervisor = Path("crates/yoctui-cli/src/daemon_bitbake.rs").read_text(encoding="utf-8")
transport = Path("crates/yoctui-protocol/src/daemon_ipc.rs").read_text(encoding="utf-8")
daemon = Path("crates/yoctui-cli/src/main.rs").read_text(encoding="utf-8")
if "mpsc::unbounded_channel()" in supervisor.split("impl Default for DaemonBitBakeSupervisor", 1)[1].split("impl DaemonBitBakeSupervisor", 1)[0]:
    raise SystemExit("BitBake event ingress regressed to an unbounded channel")
for required in (
    "BITBAKE_RELIABLE_EVENT_CAPACITY", "BITBAKE_COSMETIC_EVENT_CAPACITY",
    "cosmetic_dropped", "reliable_waits", "bitbake_event_is_cosmetic",
):
    if required not in supervisor:
        raise SystemExit(f"bounded supervisor contract is missing: {required}")
for required in ("pub fn is_readable", "pub fn send_encoded_frame_with_timeout"):
    if required not in transport:
        raise SystemExit(f"bounded daemon transport contract is missing: {required}")
for required in (
    "MAX_DAEMON_CLIENT_EVENTS_PER_TICK: usize = 32",
    "connection.is_readable()?", "send_encoded_frame_with_timeout(",
    "Duration::from_millis(2)", "Duration::from_secs(1)",
    "slow_client_disconnects", "forced_client_resynchronizations",
):
    if required not in daemon:
        raise SystemExit(f"slow-client isolation contract is missing: {required}")
print("bounded IPC source contracts valid")
PY
  cargo test -q -p yoctui --bin yoctui bounded_priority_ingress_drops_only_cosmetic_events
  cargo test -q -p yoctui --bin yoctui daemon_compatibility_cancellation_preempts_event_flood_and_terminates_once
  cargo test -q -p yoctui-protocol daemon_ipc_readiness_is_nonblocking_and_observes_peer_input
}

case "$mode" in
  --event-flood)
    verify_backpressure
    ;;
  --backpressure)
    verify_source_and_unit_contracts
    verify_backpressure
    ;;
  all)
    verify_source_and_unit_contracts
    verify_backpressure
    ;;
  *)
    printf 'unknown IPC continuity verification mode: %s\n' "$mode" >&2
    exit 2
    ;;
esac
