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
