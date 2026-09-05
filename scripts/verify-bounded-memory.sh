#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

mode="${1:-all}"

verify_contracts() {
  python3 -m unittest scripts/test_bounded_memory_harness.py
  python3 - <<'PY'
from pathlib import Path

model = Path("crates/yoctui-model/src/lib.rs").read_text(encoding="utf-8")
protocol = Path("crates/yoctui-protocol/src/daemon.rs").read_text(encoding="utf-8")
pty = Path("crates/yoctui-model/src/pty_session.rs").read_text(encoding="utf-8")
for value in ("MAX_ACTIVE_TASKS: usize = 4_096", "MAX_COMPLETED_TASKS: usize = 1_024", "HOST_TELEMETRY_HISTORY_SAMPLES: usize = 60"):
    if value not in model:
        raise SystemExit(f"model retention contract missing: {value}")
for value in ("MAX_RETAINED_EVENTS: usize = 65_536", "MAX_DAEMON_BUILD_EVENTS: usize = 2_048"):
    if value not in protocol:
        raise SystemExit(f"protocol retention contract missing: {value}")
for value in ("MAX_PTY_SCROLLBACK_LINES: usize = 100_000", "MAX_PTY_SCROLLBACK_BYTES: usize = 16 * 1024 * 1024"):
    if value not in pty:
        raise SystemExit(f"PTY retention contract missing: {value}")
print("memory retention source contracts valid")
PY
  cargo test -q -p yoctui-model high_volume_logs_remain_within_retention_limits
  cargo test -q -p yoctui-model task_event_flood_bounds_active_and_completed_state_without_losing_terminal_failure
  cargo test -q -p yoctui-model bounded_telemetry_history_retains_only_the_latest_valid_samples
  cargo test -q -p yoctui-protocol daemon_snapshot_is_gap_free_bounded_and_replays_only_retained_events
  cargo test -q -p yoctui-protocol next_generation_pty_screen_is_bounded_and_retained_for_reattach
}

verify_artifact() {
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess

root = Path("artifacts/performance/memory")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.bounded-memory-manifest.v1":
    raise SystemExit("bounded-memory manifest schema is unsupported")
revision = manifest.get("source_base_revision")
subprocess.run(["git", "merge-base", "--is-ancestor", revision, "HEAD"], check=True,
               stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
for source, digest in manifest["sources"].items():
    if hashlib.sha256(Path(source).read_bytes()).hexdigest() != digest:
        raise SystemExit(f"bounded-memory source digest mismatch: {source}")
artifact = root / manifest["artifact"]
if hashlib.sha256(artifact.read_bytes()).hexdigest() != manifest["artifact_sha256"]:
    raise SystemExit("bounded-memory artifact digest mismatch")
record = json.loads(artifact.read_text(encoding="utf-8"))
if record.get("schema") != "yoctui.performance.bounded-memory.v1":
    raise SystemExit("bounded-memory artifact schema is unsupported")
if record.get("source_base_revision") != revision:
    raise SystemExit("bounded-memory source identity mismatch")
configuration = record["configuration"]
if configuration["warmup_seconds"] != 10 or configuration["sample_window_seconds"] != 1800:
    raise SystemExit("retained evidence is not the 30-minute endurance scenario")
if configuration["sample_interval_seconds"] != 1 or configuration["event_rate_per_second"] != 4000:
    raise SystemExit("retained endurance cadence changed")
if not configuration["endurance_release_evidence"]:
    raise SystemExit("retained evidence is not labeled as release endurance")
for role in ("daemon", "client"):
    summary = record["summary"][role]
    if summary["rss_growth_bytes"] > 32 * 1024 * 1024:
        raise SystemExit(f"{role} retained RSS growth exceeds 32 MiB")
    if summary["final_window_minutes"] != 20 or summary["final_window_slope_bytes_per_minute"] > 64 * 1024:
        raise SystemExit(f"{role} retained RSS slope exceeds 64 KiB/min")
    if summary["threads_max"] > summary["threads_initial"]:
        raise SystemExit(f"{role} retained thread count grew")
retention = record["retention"]
if not retention["critical_retention_passed"] or not retention["strict_event_order"] or not retention["connection_continuity"]:
    raise SystemExit("retained endurance lost correctness or continuity")
print("30-minute bounded-memory evidence valid")
PY
}

verify_dynamic() {
  artifact="$(mktemp /tmp/yoctui-memory-gate.XXXXXX.json)"
  trap 'rm -f "$artifact"' RETURN
  ./scripts/measure-bounded-memory.py \
    --binary target/release/yoctui \
    --warmup-seconds 10 \
    --sample-seconds 60 \
    --rate 4000 \
    --output "$artifact" >/dev/null
  python3 - "$artifact" <<'PY'
import json
import sys

record = json.load(open(sys.argv[1], encoding="utf-8"))
for role in ("daemon", "client"):
    summary = record["summary"][role]
    if summary["rss_growth_bytes"] > 32 * 1024 * 1024 or summary["threads_max"] > summary["threads_initial"]:
        raise SystemExit(f"dynamic {role} resource bound failed")
if not all(record["retention"][key] for key in ("critical_retention_passed", "strict_event_order", "connection_continuity")):
    raise SystemExit("dynamic memory run lost correctness")
print("one-minute bounded-memory fixture valid")
PY
  trap - RETURN
  rm -f "$artifact"
}

case "$mode" in
  --retained)
    verify_artifact
    ;;
  --dynamic)
    verify_contracts
    verify_dynamic
    ;;
  all)
    verify_contracts
    verify_artifact
    verify_dynamic
    ;;
  *)
    printf 'unknown bounded-memory verification mode: %s\n' "$mode" >&2
    exit 2
    ;;
esac
