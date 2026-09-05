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

verify_profiles() {
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess

root = Path("artifacts/performance/profiles")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.runtime-profile-manifest.v1":
    raise SystemExit("runtime profile manifest schema is missing or unsupported")
expected = {
    "idle-daemon", "idle-client", "active-real-poky-build", "log-heavy",
    "task-event-heavy", "pty-idle", "pty-active",
}

profiles = manifest.get("profiles")
if not isinstance(profiles, dict) or set(profiles) != expected:
    raise SystemExit("runtime profile manifest does not contain the exact scenario set")
revisions = set()
binary_hashes = set()
for scenario in sorted(expected):
    report_path = root / f"{scenario}.json"
    flat_path = root / f"{scenario}.perf.txt"
    svg_path = root / f"{scenario}.svg"
    report = json.loads(report_path.read_text(encoding="utf-8"))
    if report.get("schema") != "yoctui.performance.runtime-profile.v1":
        raise SystemExit(f"{scenario}: unsupported runtime profile schema")
    if report.get("scenario") != scenario:
        raise SystemExit(f"{scenario}: scenario identity mismatch")
    sampling = report.get("sampling", {})
    if sampling.get("event") != "cycles:u" or sampling.get("frequency_hz") != 499:
        raise SystemExit(f"{scenario}: sampling configuration mismatch")
    if sampling.get("call_graph") != "lbr" or sampling.get("duration_seconds", 0) < 15:
        raise SystemExit(f"{scenario}: LBR capture or duration is invalid")
    if sampling.get("samples", 0) < 50:
        raise SystemExit(f"{scenario}: too few samples")
    quality_limit = sampling.get("maximum_dropped_unresolved_ppm")
    expected_limit = 15_000 if scenario == "active-real-poky-build" else 5_000
    if quality_limit != expected_limit:
        raise SystemExit(f"{scenario}: profile quality ceiling mismatch")
    if sampling.get("dropped_unresolved_ppm", 1_000_000) > quality_limit:
        raise SystemExit(f"{scenario}: unresolved stacks exceed the declared ceiling")
    artifacts = report.get("artifacts", {})
    if hashlib.sha256(svg_path.read_bytes()).hexdigest() != artifacts.get("svg_sha256"):
        raise SystemExit(f"{scenario}: SVG digest mismatch")
    if hashlib.sha256(flat_path.read_bytes()).hexdigest() != artifacts.get("flat_report_sha256"):
        raise SystemExit(f"{scenario}: flat report digest mismatch")
    if not report.get("top_self_symbols") or not report.get("processes"):
        raise SystemExit(f"{scenario}: symbols or process identities are absent")
    revision = report.get("revision")
    subprocess.run(
        ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
        check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    revisions.add(revision)
    binary_hashes.add(report.get("binary", {}).get("sha256"))
    declaration = profiles[scenario]
    if declaration.get("report_sha256") != hashlib.sha256(report_path.read_bytes()).hexdigest():
        raise SystemExit(f"{scenario}: report digest mismatch")
if len(revisions) != 1 or len(binary_hashes) != 1:
    raise SystemExit("runtime profiles mix revisions or binaries")
if not manifest.get("findings"):
    raise SystemExit("runtime profile findings are absent")
print(f"performance profiles valid: {len(expected)} scenarios at {next(iter(revisions))[:12]}")
PY
}

verify_wakeups() {
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess

root = Path("artifacts/performance/wakeups")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.wakeup-audit.v1":
    raise SystemExit("wakeup audit schema is missing or unsupported")
revision = manifest.get("revision")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
if manifest.get("binary_sha256") != "861da8bda754740e6a7a41675c5fc413223e16f7badce4edb9d9d3ef34ccc0f5":
    raise SystemExit("wakeup audit does not use the exact pre-optimization binary")
if manifest.get("terminal") != {"columns": 160, "rows": 50, "refresh_milliseconds": 100}:
    raise SystemExit("wakeup audit terminal configuration does not match the contract")
measurement = manifest.get("measurement", {})
if measurement.get("warmup_seconds") != 10 or measurement.get("sample_window_seconds") != 60:
    raise SystemExit("wakeup audit window does not match the robust baseline window")
if measurement.get("cpu_statistic") != "10_percent_trimmed_mean":
    raise SystemExit("wakeup audit robust statistic is absent")
for available, reason in (
    ("scheduler_tracepoints_available", "scheduler_tracepoints_reason"),
    ("process_wakeup_counter_available", "process_wakeup_counter_reason"),
    ("strace_attach_available", "strace_attach_reason"),
):
    if measurement.get(available) is not False or not measurement.get(reason):
        raise SystemExit(f"wakeup audit must explain unavailable evidence: {available}")
artifacts = manifest.get("artifacts", {})
paths = {
    "idle-attached-perf-stat.csv": root / "idle-attached-perf-stat.csv",
    "idle-attached-strace.txt": root / "idle-attached-strace.txt",
    "idle-daemon-baseline.json": Path("artifacts/performance/baseline/idle-daemon.json"),
    "idle-attached-client-baseline.json": Path("artifacts/performance/baseline/idle-attached-client.json"),
}

if set(artifacts) != set(paths):
    raise SystemExit("wakeup audit artifact set is incomplete")
for name, path in paths.items():
    if hashlib.sha256(path.read_bytes()).hexdigest() != artifacts[name]:
        raise SystemExit(f"wakeup audit artifact digest mismatch: {name}")
required_categories = {
    "ui_tick_render", "animation", "client_telemetry", "daemon_telemetry",
    "daemon_listener", "supervisor_polling", "client_ipc_polling", "reconnect",
    "operation_polling", "pty_screen", "logs_jobs_status", "ipc_heartbeat",
    "bitbake_backend",
}
sources = manifest.get("timer_sources", [])
if {source.get("category") for source in sources} != required_categories:
    raise SystemExit("wakeup audit timer-source catalog is incomplete")
for source in sources:
    required = {"source", "cadence", "guard", "idle_behavior", "periodic_when_idle", "owner"}
    if not required.issubset(source):
        raise SystemExit(f"wakeup source is incomplete: {source.get('category')}")
    if not source["source"].startswith("crates/"):
        raise SystemExit(f"wakeup source lacks a repository code location: {source['category']}")
observations = manifest.get("observations", {})
for key in (
    "idle_daemon_voluntary_context_switches_per_second",
    "idle_attached_perf_context_switches_per_second",
    "idle_render_invocations_per_second",
    "daemon_telemetry_publications_per_second",
    "client_telemetry_polls_per_second",
):
    if observations.get(key, 0) <= 0:
        raise SystemExit(f"wakeup audit observation is absent: {key}")
if not manifest.get("findings"):
    raise SystemExit("wakeup audit findings are absent")
print(f"wakeup audit valid: {len(sources)} timer sources at {revision[:12]}")
PY
}

verify_event_loops() {
  python3 - <<'PY'
from pathlib import Path

daemon = Path("crates/yoctui-cli/src/main.rs").read_text(encoding="utf-8")
ipc = Path("crates/yoctui-protocol/src/daemon_ipc.rs").read_text(encoding="utf-8")
if "listener.accept(Duration::from_millis(1))" in daemon:
    raise SystemExit("daemon listener regressed to one-millisecond polling")
if "thread::sleep(CONNECT_RETRY_INTERVAL.min(timeout))" in ipc.split("impl DaemonListener", 1)[1].split("impl Drop", 1)[0]:
    raise SystemExit("daemon listener regressed to sleep-based readiness polling")
if "terminal.draw(|f| render(f, &app))?;\n        if event::poll" in daemon:
    raise SystemExit("interactive client regressed to unconditional idle rendering")
if "if build_jobs.active_job_id().is_some()" not in daemon:
    raise SystemExit("idle client must not poll the inactive local BitBake backend")
print("event-loop source contracts valid")
PY
  cargo build -q -p yoctui --bin yoctui
  cargo test -q -p yoctui-protocol daemon_ipc
  cargo test -q -p yoctui --bin yoctui idle_daemon_waits_without_delaying_attached_or_active_work
  python3 scripts/test-idle-event-loops.py --binary target/debug/yoctui
}

verify_render() {
  python3 - <<'PY'
from pathlib import Path

source = Path("crates/yoctui-cli/src/main.rs").read_text(encoding="utf-8")
scheduler = Path("crates/yoctui-cli/src/render_scheduler.rs").read_text(encoding="utf-8")
tui = source.split("async fn tui(", 1)[1].split("fn termination_receiver", 1)[0]
draw = "terminal.draw(|f| render(f, &app))?;"
if tui.count(draw) != 1:
    raise SystemExit("interactive runtime must have exactly one centralized render call")
guarded = "if render_scheduler.take_frame() {\n            " + draw
if guarded not in tui:
    raise SystemExit("interactive render call is not guarded by coalesced invalidation")
for required in (
    "RenderCause::Input", "RenderCause::State", "RenderCause::Telemetry",
    "RenderCause::Presentation", "RenderCause::Resize",
    "interactive_frame_interval(refresh)",
):
    if required not in tui:
        raise SystemExit(f"render invalidation source is missing: {required}")
for required in ("requests", "frames", "coalesced", "skipped_checks"):
    if required not in scheduler:
        raise SystemExit(f"render scheduler metric is missing: {required}")
print("dirty render source contracts valid")
PY
  cargo test -q -p yoctui --bin yoctui render_scheduler
  cargo test -q -p yoctui --bin yoctui normal_render_interval_is_capped_at_ten_hertz
}

verify_animations() {
  python3 - <<'PY'
from pathlib import Path

source = Path("crates/yoctui-cli/src/main.rs").read_text(encoding="utf-8")
scheduler = Path("crates/yoctui-cli/src/render_scheduler.rs").read_text(encoding="utf-8")
tui = source.split("async fn tui(", 1)[1].split("fn termination_receiver", 1)[0]
for required in (
    "has_visible_indeterminate_activity(&app)",
    "presentation_now + ANIMATION_INTERVAL",
    "presentation_now + ELAPSED_REFRESH_INTERVAL",
):
    if required not in tui:
        raise SystemExit(f"animation scheduler contract is missing: {required}")
if "Action::Tick" not in tui:
    raise SystemExit("visible animation does not advance the model phase")
for required in (
    "app.reduced_motion", "app.active_dialog().is_some()",
    "Screen::Dashboard | Screen::Tasks", "TaskState::Active",
    "task.progress.is_none()",
):
    if required not in scheduler:
        raise SystemExit(f"animation visibility guard is missing: {required}")
print("visible-only animation source contracts valid")
PY
  cargo test -q -p yoctui --bin yoctui render_scheduler::tests::animation_is_visible_only_indeterminate_and_nonterminal
  cargo test -q -p yoctui --bin yoctui render_scheduler::tests::overlays_and_reduced_motion_freeze_animation_but_not_elapsed_time
  cargo test -q -p yoctui --bin yoctui render_scheduler::tests::presentation_cadences_are_explicitly_bounded
  cargo test -q -p yoctui-ui active_task_indicator_uses_braille_motion_and_accessible_fallbacks
}

verify_telemetry() {
  python3 - <<'PY'
from pathlib import Path

source = Path("crates/yoctui-cli/src/main.rs").read_text(encoding="utf-8")
scheduler = Path("crates/yoctui-cli/src/telemetry_scheduler.rs").read_text(encoding="utf-8")
sampler = source.split("struct HostTelemetrySampler", 1)[1].split("impl Drop for TerminalGuard", 1)[0]
if "ProcessCommand" in sampler or "Command::new" in sampler:
    raise SystemExit("host telemetry must not spawn a process per sample")
for required in (
    "CLIENT_VISIBLE_INTERVAL", "CLIENT_BACKGROUND_INTERVAL",
    "DAEMON_ACTIVE_INTERVAL", "DAEMON_ATTACHED_IDLE_INTERVAL",
    "connected_clients == 0", "Screen::Dashboard | Screen::Tasks",
):
    if required not in scheduler:
        raise SystemExit(f"telemetry demand/cadence guard is missing: {required}")
for required in (
    "build_disk_device: Option<(u64, u64)>",
    "network_interface: Option<String>",
    "logical_cpu_count: Option<u16>",
    "telemetry_visible && telemetry_changed",
):
    if required not in source:
        raise SystemExit(f"telemetry cache/invalidation contract is missing: {required}")
print("demand-aware telemetry source contracts valid")
PY
  cargo test -q -p yoctui --bin yoctui telemetry_scheduler
  cargo test -q -p yoctui --bin yoctui telemetry_sampling
  cargo test -q -p yoctui-model bounded_telemetry_history_retains_only_the_latest_valid_samples
}

verify_logs() {
  python3 - <<'PY'
from pathlib import Path

model = Path("crates/yoctui-model/src/lib.rs").read_text(encoding="utf-8")
runtime = Path("crates/yoctui-cli/src/client_runtime.rs").read_text(encoding="utf-8")
app = Path("crates/yoctui-app/src/lib.rs").read_text(encoding="utf-8")
filtered = model.split("pub fn filtered(&self)", 1)[1].split("pub fn diagnostics", 1)[0]
if "e.message.to_lowercase()" in filtered:
    raise SystemExit("log filtering regressed to lowercasing every retained message")
for required in (
    "normalized_messages: VecDeque<String>", "pub fn insert_batch",
    "Action::Logs", "maximum_horizontal_offset", "for entry in self.filtered()",
):
    if required not in model:
        raise SystemExit(f"bounded log projection contract is missing: {required}")
for required in ("pending_logs", "flush_log_events", "MAX_EVENTS_PER_POLL"):
    if required not in runtime:
        raise SystemExit(f"client log batching contract is missing: {required}")
if "apply_log_events_to_app" not in app:
    raise SystemExit("daemon replica lacks ordered batch log reduction")
print("bounded batched log source contracts valid")
PY
  cargo test -q -p yoctui-model tests::log_batches_preserve_critical_order_counts_and_cached_search -- --exact
  cargo test -q -p yoctui-model tests::log_retention_prefers_important_diagnostics_and_reports_coalescing -- --exact
  cargo test -q -p yoctui-model tests::ux_logs_virtualized_window_and_source_time_filters_stay_bounded -- --exact
  cargo test -q -p yoctui-app tests::daemon_client_batches_contiguous_logs_with_one_model_install -- --exact
  cargo test -q -p yoctui-ui tests::log_workspace_exposes_search_filters_pressure_and_narrow_wrap_safely -- --exact
}

verify_tasks() {
  python3 - <<'PY'
from pathlib import Path

model = Path("crates/yoctui-model/src/lib.rs").read_text(encoding="utf-8")
runtime = Path("crates/yoctui-cli/src/client_runtime.rs").read_text(encoding="utf-8")
app = Path("crates/yoctui-app/src/lib.rs").read_text(encoding="utf-8")
for required in (
    "pub enum TaskEvent", "Action::TaskEvents", "apply_task_batch",
    "task_progress_coalesced", "TaskProjectionCache", "MAX_ACTIVE_TASKS",
):
    if required not in model:
        raise SystemExit(f"bounded task update contract is missing: {required}")
for required in ("pending_tasks", "flush_task_events", "MAX_EVENTS_PER_POLL"):
    if required not in runtime:
        raise SystemExit(f"client task batching contract is missing: {required}")
if "apply_task_events_to_app" not in app:
    raise SystemExit("daemon replica lacks ordered batch task reduction")
print("bounded batched task source contracts valid")
PY
  cargo test -q -p yoctui-model tests::task_batches_coalesce_progress_and_preserve_terminal_failures -- --exact
  cargo test -q -p yoctui-model tests::task_event_flood_bounds_active_and_completed_state_without_losing_terminal_failure -- --exact
  cargo test -q -p yoctui-model tests::unchanged_task_projection_reuses_sorted_identity_cache -- --exact
  cargo test -q -p yoctui-app tests::daemon_client_batches_task_progress_without_losing_failure -- --exact
  cargo test -q -p yoctui --bin yoctui client_runtime
}

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
daemon = Path("crates/yoctui-cli/src/main.rs").read_text(encoding="utf-8")
for required in (
    "snapshot_bytes_upper_bound", "snapshot_serializations", "synchronize_bounded",
):
    if required not in protocol:
        raise SystemExit(f"IPC snapshot/replay contract is missing: {required}")
if "send_encoded_frame" not in transport or "encoded_event_frames" not in daemon:
    raise SystemExit("shared daemon fan-out encoding contract is missing")
print(
    "IPC audit valid: "
    f"{wire['frames_per_second']:.1f} frames/s, "
    f"{wire['bytes_per_second'] / 1024:.1f} KiB/s, zero replacement snapshots"
)
PY
  cargo test -q -p yoctui-protocol daemon_snapshot_is_gap_free_bounded_and_replays_only_retained_events
  cargo test -q -p yoctui-protocol daemon_journal_uses_conservative_headroom_between_snapshot_serializations
  cargo test -q -p yoctui-protocol daemon_ipc_sends_one_preencoded_frame_without_reserialization
  cargo test -q -p yoctui --bin yoctui daemon_live_event_replay_is_bounded_below_client_poll_capacity
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

verify_scheduling() {
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess

root = Path("artifacts/performance/scheduling")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.scheduling-manifest.v1":
    raise SystemExit("scheduling manifest schema is missing or unsupported")
revision = manifest.get("source_base_revision")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
artifact = root / manifest["artifact"]
if hashlib.sha256(artifact.read_bytes()).hexdigest() != manifest["artifact_sha256"]:
    raise SystemExit("scheduling evidence digest mismatch")
record = json.loads(artifact.read_text(encoding="utf-8"))
if record.get("schema") != "yoctui.performance.scheduling-audit.v1":
    raise SystemExit("scheduling evidence schema is unsupported")
if record.get("revision") != revision:
    raise SystemExit("scheduling evidence source identity mismatch")
configuration = record["configuration"]
if configuration.get("repetitions") != 3 or configuration.get("probe_interval_ms") != 10:
    raise SystemExit("scheduling evidence does not use the reviewed repeated method")
affinity = record["host"]["affinity_cpus"]
if affinity < 1 or record["host"].get("cgroup_v2") is not True:
    raise SystemExit("scheduling host identity is incomplete")
for name in ("inherited_nice_0", "deprioritized_nice_5", "cpu_weight_200"):
    scenario = record["scenarios"].get(name)
    loads = record["saturation"].get(name)
    if scenario is None or len(scenario["trials"]) != 3 or len(loads) != 3:
        raise SystemExit(f"scheduling scenario is incomplete: {name}")
    for load in loads:
        if load["status"] != "completed" or load["children_reaped"] is not True:
            raise SystemExit(f"scheduling load did not clean up: {name}")
        if len(load["selected_cpus"]) != affinity or not load["default_saturates_full_affinity"]:
            raise SystemExit(f"scheduling load left a deliberate free CPU: {name}")
        if load["minimum_worker_cpu_percent"] < 25:
            raise SystemExit(f"scheduling load was below its declared minimum: {name}")
if any(trial["process"]["nice"] != 0 for trial in record["scenarios"]["inherited_nice_0"]["trials"]):
    raise SystemExit("inherited scheduling evidence did not run at nice 0")
if any(trial["process"]["nice"] != 5 for trial in record["scenarios"]["deprioritized_nice_5"]["trials"]):
    raise SystemExit("deprioritized scheduling evidence did not run at nice 5")
if any(trial["process"]["cpu_weight"] != 200 for trial in record["scenarios"]["cpu_weight_200"]["trials"]):
    raise SystemExit("user-service scheduling evidence did not apply CPUWeight=200")
if record["capabilities"].get("negative_nice_unprivileged") is not False:
    raise SystemExit("evidence must not claim unprivileged negative nice is portable")
baseline = record["scenarios"]["inherited_nice_0"]["summary"]["median_p95_wake_latency_ms"]
weighted = record["scenarios"]["cpu_weight_200"]["summary"]["median_p95_wake_latency_ms"]
if baseline > 100:
    raise SystemExit("normal inherited priority failed the responsiveness contract")
decision = " ".join(manifest.get("decision", {}).values())
for forbidden in ("require root", "require realtime"):
    if forbidden in decision.lower():
        raise SystemExit(f"scheduling guidance contains forbidden policy: {forbidden}")
print(
    "scheduling audit valid: inherited median p95 "
    f"{baseline:.4f} ms; CPUWeight=200 {weighted:.4f} ms; no mandatory tuning"
)
PY

  python3 -m unittest scripts/test_scheduler_latency_probe.py
  current="$(mktemp /tmp/yoctui-scheduling-current.XXXXXX.json)"
  trap 'unlink "$current" 2>/dev/null || true' RETURN
  ./scripts/measure-scheduling.py \
    --revision "$(git rev-parse HEAD)" \
    --duration-seconds 1 \
    --repetitions 1 \
    --output "$current" >/dev/null
  python3 - "$current" <<'PY'
from pathlib import Path
import json
import os
import sys

record = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
inherited = record["scenarios"]["inherited_nice_0"]
if inherited["summary"]["median_p95_wake_latency_ms"] > 100:
    raise SystemExit("current inherited-priority scheduler latency exceeds 100 ms")
if inherited["trials"][0]["process"]["nice"] != 0:
    raise SystemExit("current default scheduling probe did not inherit nice 0")
available = len(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else os.cpu_count()
for loads in record["saturation"].values():
    if any(len(load["selected_cpus"]) != available for load in loads):
        raise SystemExit("current scheduling measurement left a deliberate free CPU")
print("current inherited-priority scheduling remains responsive under full-affinity load")
PY
  trap - RETURN
  unlink "$current"
}

case "$mode" in
  --contract)
    verify_contract
    ;;
  --baseline)
    verify_contract
    verify_baseline
    ;;
  --profiles)
    verify_contract
    verify_baseline
    verify_profiles
    ;;
  --wakeups)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    ;;
  --event-loops)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    ;;
  --render)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    ;;
  --animations)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    ;;
  --telemetry)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    ;;
  --logs)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    ;;
  --tasks)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    ;;
  --ipc)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    verify_ipc
    ;;
  --tokio)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    verify_ipc
    verify_tokio
    ;;
  --scheduling)
    verify_contract
    verify_baseline
    verify_profiles
    verify_wakeups
    verify_event_loops
    verify_render
    verify_animations
    verify_telemetry
    verify_logs
    verify_tasks
    verify_ipc
    verify_tokio
    verify_scheduling
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
