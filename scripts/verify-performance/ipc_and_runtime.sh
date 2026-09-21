verify_ipc() {
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess

root = Path("artifacts/performance/ipc")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.ipc-audit.v1":
    raise SystemExit("IPC audit manifest schema is missing or unsupported")
revision = manifest.get("source_base_revision")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
artifact = root / "event-flood-incremental.json"
if hashlib.sha256(artifact.read_bytes()).hexdigest() != manifest.get("artifact_sha256"):
    raise SystemExit("IPC audit artifact digest mismatch")
record = json.loads(artifact.read_text(encoding="utf-8"))
if record.get("schema") != "yoctui.performance.event-flood-observation.v1":
    raise SystemExit("IPC event-flood evidence schema is unsupported")
if record.get("identity", {}).get("source_base_revision") != revision:
    raise SystemExit("IPC evidence source identity mismatch")
if record.get("identity", {}).get("binary_sha256") != manifest.get("binary_sha256"):
    raise SystemExit("IPC evidence binary identity mismatch")
configuration = record.get("configuration", {})
if configuration.get("rate_events_per_second") != 2_000:
    raise SystemExit("IPC audit must exercise 2,000 events/s")
if configuration.get("duration_seconds") != 2.0:
    raise SystemExit("IPC audit duration mismatch")
client = record.get("client", {})
wire = client.get("wire_metrics", {})
if client.get("snapshot_replacements") != 0 or client.get("resync_requests") != 0:
    raise SystemExit("attached IPC regressed to redundant snapshot replacement")
if client.get("event_sequences_strictly_increasing") is not True:
    raise SystemExit("incremental IPC ordering is not strict")
if client.get("connection_continuity") is not True:
    raise SystemExit("IPC audit client disconnected")
if wire.get("initial_snapshot_json_bytes", 0) <= 0:
    raise SystemExit("IPC audit omitted snapshot size")
event_wire = wire.get("received_by_type", {}).get("event", {})
for field in ("frames", "frame_bytes", "minimum_frame_bytes", "maximum_frame_bytes"):
    if event_wire.get(field, 0) <= 0:
        raise SystemExit(f"IPC audit omitted incremental event {field}")
for field in ("frames_per_second", "bytes_per_second", "daemon_cpu_seconds"):
    if wire.get(field) is None:
        raise SystemExit(f"IPC audit omitted {field}")
if wire["bytes_per_second"] >= 100_000:
    raise SystemExit("incremental IPC traffic exceeds the audited 100 KiB/s ceiling")
if record.get("bounds", {}).get("supervisor_ingress") != "unbounded_pre_backpressure":
    raise SystemExit("IPC audit must identify the remaining upstream boundary")
if record.get("result", {}).get("expected_pre_backpressure_terminal_starvation_observed") is not True:
    raise SystemExit("IPC audit must not claim the later backpressure task already passes")

profile = json.loads(Path("artifacts/performance/profiles/task-event-heavy.json").read_text(encoding="utf-8"))
hot = profile.get("top_self_symbols", [])
if not any("format_escaped_str" in item.get("symbol", "") and item.get("self_percent", 0) >= 30 for item in hot):
    raise SystemExit("IPC optimization is not tied to the captured serialization hot path")

protocol = Path("crates/yoctui-protocol/src/daemon.rs").read_text(encoding="utf-8")
transport = Path("crates/yoctui-protocol/src/daemon_ipc.rs").read_text(encoding="utf-8")
daemon = Path("crates/yoctui-cli/src/daemon_server.rs").read_text(encoding="utf-8")
supervisor_root = Path("crates/yoctui-cli/src/daemon_bitbake")
supervisor = Path("crates/yoctui-cli/src/daemon_bitbake.rs").read_text(encoding="utf-8")
supervisor += "".join(path.read_text(encoding="utf-8") for path in sorted(supervisor_root.rglob("*.rs")))
for required in (
    "snapshot_bytes_upper_bound", "snapshot_serializations", "synchronize_bounded",
):
    if required not in protocol:
        raise SystemExit(f"IPC snapshot/replay contract is missing: {required}")
if "send_encoded_frame" not in transport or "encoded_event_frames" not in daemon:
    raise SystemExit("shared daemon fan-out encoding contract is missing")
if "wait_for_activity_with_additional_fd" not in transport or "notification_fd()" not in daemon:
    raise SystemExit("event-driven daemon readiness contract is missing")
for required in (
    "ActivityNotification", "signal_batched", "consume_notification",
    "bitbake_event_requires_immediate_wake",
):
    if required not in supervisor:
        raise SystemExit(f"coalesced BitBake readiness contract is missing: {required}")
print(
    "IPC audit valid: "
    f"{wire['frames_per_second']:.1f} frames/s, "
    f"{wire['bytes_per_second'] / 1024:.1f} KiB/s, zero replacement snapshots"
)
PY
  cargo test -q -p yoctui-protocol daemon_snapshot_is_gap_free_bounded_and_replays_only_retained_events
  cargo test -q -p yoctui-protocol daemon_journal_uses_conservative_headroom_between_snapshot_serializations
  cargo test -q -p yoctui-protocol daemon_ipc_sends_one_preencoded_frame_without_reserialization
  cargo test -q -p yoctui-protocol daemon_listener_wait_wakes_for_additional_readiness_fd
  cargo test -q -p yoctui --bin yoctui daemon_live_event_replay_is_bounded_below_client_poll_capacity
  cargo test -q -p yoctui --bin yoctui activity_notification_coalesces_and_rearms
}

verify_tokio() {
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess

root = Path("artifacts/performance/tokio")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.tokio-audit.v1":
    raise SystemExit("Tokio audit manifest schema is missing or unsupported")
revision = manifest.get("source_base_revision")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
for phase in ("pre_optimization", "post_optimization"):
    expected = manifest[phase]
    artifact = root / expected["path"]
    if hashlib.sha256(artifact.read_bytes()).hexdigest() != expected["sha256"]:
        raise SystemExit(f"Tokio {phase} artifact digest mismatch")
    record = json.loads(artifact.read_text(encoding="utf-8"))
    if record.get("schema") != "yoctui.performance.tokio-runtime.v1":
        raise SystemExit(f"Tokio {phase} artifact schema is unsupported")
    measurement = record["measurement"]
    workers = sum(thread["name"] == "tokio-rt-worker" for thread in measurement["threads"])
    if workers != expected["runtime_workers"]:
        raise SystemExit(f"Tokio {phase} worker count mismatch")
    if measurement["thread_count_start"] != expected["process_threads"]:
        raise SystemExit(f"Tokio {phase} initial thread count mismatch")
    if measurement["thread_count_end"] != expected["process_threads"]:
        raise SystemExit(f"Tokio {phase} leaked a thread during the idle sample")
inventory = json.loads((root / manifest["pre_optimization"]["path"]).read_text(encoding="utf-8"))["source_inventory"]
if inventory != manifest["source_inventory_before"]:
    raise SystemExit("Tokio source inventory evidence mismatch")

main = Path("crates/yoctui-cli/src/main.rs").read_text(encoding="utf-8")
main += Path("crates/yoctui-cli/src/input_routing.rs").read_text(encoding="utf-8")
main += Path("crates/yoctui-cli/src/tests/cli/tokio_runtime_two_workers_isolate_a_bounded_blocking_poll.rs").read_text(encoding="utf-8")
if "#[tokio::main(worker_threads = 2)]" not in main:
    raise SystemExit("Yoctui runtime is not pinned to the audited two-worker policy")
for required in (
    "tokio_runtime_two_workers_isolate_a_bounded_blocking_poll",
    "spawn_blocking",
    "MAX_NORMAL_RENDER_RATE",
):
    if required not in main:
        raise SystemExit(f"Tokio scheduling contract is missing: {required}")
print("Tokio audit valid: idle runtime reduced from 8 workers/9 threads to 2 workers/3 threads")
PY

  cargo build -q -p yoctui --bin yoctui
  current="$(mktemp /tmp/yoctui-tokio-current.XXXXXX.json)"
  saturation="$(mktemp /tmp/yoctui-tokio-saturation.XXXXXX.json)"
  event_log="$(mktemp /tmp/yoctui-tokio-saturation.XXXXXX.jsonl)"
  load_pid=""
  cleanup_tokio_fixture() {
    if [[ -n "$load_pid" ]] && kill -0 "$load_pid" 2>/dev/null; then
      kill "$load_pid" 2>/dev/null || true
      wait "$load_pid" 2>/dev/null || true
    fi
    unlink "$current" "$saturation" "$event_log" 2>/dev/null || true
  }
  trap cleanup_tokio_fixture RETURN

  ./scripts/measure-tokio-runtime.py \
    --binary target/debug/yoctui \
    --revision "$(git rev-parse HEAD)" \
    --sample-seconds 2 \
    --output "$current" >/dev/null
  python3 - "$current" <<'PY'
from pathlib import Path
import json
import sys

record = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
measurement = record["measurement"]
names = [thread["name"] for thread in measurement["threads"]]
if measurement["thread_count_start"] != 3 or measurement["thread_count_end"] != 3:
    raise SystemExit("current idle daemon does not retain the audited three-thread bound")
if names.count("tokio-rt-worker") != 2:
    raise SystemExit("current idle daemon does not have exactly two runtime workers")
if any(name.startswith("tokio-blocking") for name in names):
    raise SystemExit("idle daemon eagerly created a blocking-pool thread")
print("current Tokio runtime valid: 2 workers, 3 stable process threads")
PY

  cargo test -q -p yoctui --bin yoctui \
    tokio_runtime_two_workers_isolate_a_bounded_blocking_poll --no-run
  ./scripts/cpu-saturation-harness.py \
    --warmup-seconds 0.25 \
    --duration-seconds 3 \
    --minimum-worker-cpu-percent 30 \
    --event-log "$event_log" \
    --output "$saturation" >/dev/null &
  load_pid="$!"
  ready=false
  for _ in $(seq 1 300); do
    if rg -q '"event":"ready"' "$event_log"; then
      ready=true
      break
    fi
    sleep 0.02
  done
  if [[ "$ready" != true ]]; then
    printf '%s\n' 'CPU saturation fixture did not become ready for Tokio test' >&2
    return 1
  fi
  cargo test -q -p yoctui --bin yoctui \
    tokio_runtime_two_workers_isolate_a_bounded_blocking_poll
  wait "$load_pid"
  load_pid=""
  python3 - "$saturation" <<'PY'
from pathlib import Path
import json
import os
import sys

record = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
available = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else os.cpu_count()
if record["status"] != "completed" or not record["cleanup"]["children_reaped"]:
    raise SystemExit("Tokio saturation fixture did not complete cleanly")
if len(record["workers"]) != available:
    raise SystemExit("Tokio saturation fixture left an affinity CPU deliberately free")
if record["achieved"]["minimum_worker_cpu_percent"] < 30:
    raise SystemExit("Tokio saturation fixture did not achieve its declared load")
print(f"Tokio reactor test passed while all {available} affinity CPUs were runnable")
PY
  trap - RETURN
  cleanup_tokio_fixture
}
