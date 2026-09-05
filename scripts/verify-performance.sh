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
