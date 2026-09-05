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

verify_latency() {
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess

root = Path("artifacts/performance/ipc-latency")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.ipc-latency-manifest.v1":
    raise SystemExit("IPC latency manifest schema is missing or unsupported")
revision = manifest.get("source_base_revision")
if not isinstance(revision, str) or len(revision) != 40:
    raise SystemExit("IPC latency revision must be an exact commit")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
artifact = root / manifest["artifact"]
if hashlib.sha256(artifact.read_bytes()).hexdigest() != manifest["artifact_sha256"]:
    raise SystemExit("IPC latency evidence digest mismatch")
record = json.loads(artifact.read_text(encoding="utf-8"))
if record.get("schema") != "yoctui.performance.ipc-latency.v1":
    raise SystemExit("IPC latency evidence schema is unsupported")
if record.get("revision") != revision:
    raise SystemExit("IPC latency evidence source identity mismatch")
if record.get("binary", {}).get("sha256") != manifest["binary_sha256"]:
    raise SystemExit("IPC latency binary identity mismatch")
if manifest.get("method") != {
    "release_profile": True,
    "warmup_seconds": 1,
    "observations_per_path": 100,
    "event_warmup_observations": 50,
    "clock": "CLOCK_MONOTONIC",
    "transport": "AF_UNIX SOCK_STREAM length-prefixed JSON",
    "load": "one pinned worker per affinity CPU; no deliberately free CPU",
}:
    raise SystemExit("IPC latency manifest method changed")
for source, digest in manifest["sources"].items():
    if hashlib.sha256(Path(source).read_bytes()).hexdigest() != digest:
        raise SystemExit(f"IPC latency source digest mismatch: {source}")
configuration = record["configuration"]
if configuration["clock"] != "CLOCK_MONOTONIC":
    raise SystemExit("IPC latency evidence did not use monotonic time")
if configuration["transport"] != "AF_UNIX SOCK_STREAM length-prefixed JSON":
    raise SystemExit("IPC latency transport identity changed")
if configuration["warmup_seconds"] != 1 or configuration["observations_per_path"] != 100:
    raise SystemExit("IPC latency evidence window changed")
if configuration["event_warmup_observations"] != 50:
    raise SystemExit("IPC latency event warmup changed")
if configuration["event_path"] != [
    "fixture_bridge", "bridge_backend", "daemon_bitbake_supervisor",
    "daemon_snapshot_journal", "unix_ipc", "attached_protocol_client",
]:
    raise SystemExit("IPC latency production event path is incomplete")
samples = record["samples"]
for name in (
    "daemon_event_to_client",
    "client_command_to_daemon",
    "cancellation_request_to_ack",
):
    if len(samples.get(name, [])) != 100:
        raise SystemExit(f"IPC latency path does not contain 100 samples: {name}")
    for sequence, sample in enumerate(samples[name], 1):
        if sample["sequence"] != sequence:
            raise SystemExit(f"IPC latency sample order changed: {name}")
        if name == "daemon_event_to_client":
            if sample["fixture_sequence"] != sequence + 50:
                raise SystemExit("fixture event sequence changed")
            if sample["emitted_ns"] > sample["received_ns"]:
                raise SystemExit("event latency timestamps are not monotonic")
        elif sample["sent_ns"] > sample["acknowledged_ns"]:
            raise SystemExit(f"command latency timestamps are not monotonic: {name}")
summary = record["summary"]
for metric in ("daemon_event_to_client_ms", "client_command_to_daemon_ms"):
    observed = summary[metric]
    if observed["p50"] < 0 or observed["p50"] > 25 or observed["p95"] > 100:
        raise SystemExit(f"ordinary IPC latency threshold failed: {metric}")
cancellation = summary["cancellation_request_to_ack_ms"]
if cancellation["p50"] < 0 or cancellation["p95"] > 250:
    raise SystemExit("cancellation acknowledgement latency threshold failed")
if summary["accepted_cancellation_requests"] < 2:
    raise SystemExit("IPC latency evidence did not prove live cancellation per batch")
continuity = record["continuity"]
if not continuity["primary_client_connected"] or not continuity["reconnect_succeeded"]:
    raise SystemExit("IPC latency evidence lost attach/reconnect continuity")
if continuity["backend_disconnect_events"] != 0:
    raise SystemExit("IPC latency evidence observed a backend disconnect")
if not continuity["protocol_sequences_strictly_increasing"]:
    raise SystemExit("IPC latency protocol ordering failed")
load = record["saturation"]
host = record["host"]
if not {"logical_cpus", "affinity_cpus", "kernel"}.issubset(host):
    raise SystemExit("IPC latency host identity is incomplete")
affinity = host["affinity_cpus"]
if not load["alive_for_every_observation"] or not load["completed_after_measurement"]:
    raise SystemExit("CPU saturation did not span every IPC observation")
if load["configuration"]["selected_cpus"] != affinity:
    raise SystemExit("IPC latency evidence deliberately left an affinity CPU free")
if load["configuration"]["requested_workers"] != len(affinity):
    raise SystemExit("IPC latency evidence did not run one worker per affinity CPU")
if load["achieved"]["minimum_worker_cpu_percent"] < 25:
    raise SystemExit("IPC latency evidence did not keep every CPU runnable")
if load["achieved"]["host_cpu_utilization_percent"] < 90:
    raise SystemExit("IPC latency host was not saturated")
if load["cleanup"]["children_reaped"] is not True:
    raise SystemExit("IPC latency load workers were not reaped")
print(
    "IPC latency valid under full-CPU load: event p95 "
    f"{summary['daemon_event_to_client_ms']['p95']:.3f} ms, command p95 "
    f"{summary['client_command_to_daemon_ms']['p95']:.3f} ms, cancellation p95 "
    f"{summary['cancellation_request_to_ack_ms']['p95']:.3f} ms"
)
PY

  python3 -m unittest scripts/test_measure_ipc_latency.py
  cargo build -q -p yoctui
  current="$(mktemp /tmp/yoctui-ipc-latency-current.XXXXXX.json)"
  trap 'unlink "$current" 2>/dev/null || true' RETURN
  ./scripts/measure-ipc-latency.py \
    --binary target/debug/yoctui \
    --revision "$(git rev-parse HEAD)" \
    --warmup-seconds 0.5 \
    --observations 100 \
    --output "$current" >/dev/null
  python3 - "$current" <<'PY'
from pathlib import Path
import json
import sys

record = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
summary = record["summary"]
for metric in ("daemon_event_to_client_ms", "client_command_to_daemon_ms"):
    if summary[metric]["p50"] > 25 or summary[metric]["p95"] > 100:
        raise SystemExit(f"current ordinary IPC latency exceeds its threshold: {metric}")
if summary["cancellation_request_to_ack_ms"]["p95"] > 250:
    raise SystemExit("current cancellation acknowledgement exceeds 250 ms")
if summary["accepted_cancellation_requests"] < 2:
    raise SystemExit("current cancellation path did not remain functional")
load = record["saturation"]
if (
    not load["alive_for_every_observation"]
    or load["achieved"]["minimum_worker_cpu_percent"] < 25
    or load["achieved"]["host_cpu_utilization_percent"] < 90
):
    raise SystemExit("current IPC latency run lacked full-CPU saturation")
print("current daemon event, command, and cancellation IPC latency remain bounded")
PY
  trap - RETURN
  unlink "$current"
}

case "$mode" in
  --event-flood)
    verify_backpressure
    ;;
  --backpressure)
    verify_source_and_unit_contracts
    verify_backpressure
    ;;
  --latency)
    verify_latency
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
