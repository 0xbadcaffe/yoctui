verify_coexistence() {
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess

root = Path("artifacts/performance/coexistence")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.bitbake-coexistence-manifest.v1":
    raise SystemExit("BitBake coexistence manifest schema is missing or unsupported")
revision = manifest.get("source_base_revision")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
artifact = root / manifest["artifact"]
if hashlib.sha256(artifact.read_bytes()).hexdigest() != manifest["artifact_sha256"]:
    raise SystemExit("BitBake coexistence evidence digest mismatch")
record = json.loads(artifact.read_text(encoding="utf-8"))
if record.get("schema") != "yoctui.performance.bitbake-coexistence.v1":
    raise SystemExit("BitBake coexistence evidence schema is unsupported")
if record.get("revision") != revision:
    raise SystemExit("BitBake coexistence evidence source identity mismatch")
configuration = record["configuration"]
if configuration.get("repetitions") != 3 or configuration.get("probe_interval_ms") != 10:
    raise SystemExit("BitBake coexistence evidence does not use the reviewed repeated method")
affinity = record["host"]["affinity_cpus"]
expected = {
    "one_worker_per_logical_cpu": len(affinity),
    "two_workers_per_logical_cpu": len(affinity) * 2,
}
for name, worker_count in expected.items():
    scenario = record["scenarios"].get(name)
    if scenario is None or scenario["workers"] != worker_count:
        raise SystemExit(f"BitBake coexistence scenario is incomplete: {name}")
    if len(scenario["trials"]) != 3 or len(scenario["saturation"]) != 3:
        raise SystemExit(f"BitBake coexistence trials are incomplete: {name}")
    if scenario["summary"]["median_p95_wake_latency_ms"] > 100:
        raise SystemExit(f"BitBake coexistence scenario exceeded responsiveness bound: {name}")
    for load in scenario["saturation"]:
        if load["status"] != "completed" or load["children_reaped"] is not True:
            raise SystemExit(f"BitBake coexistence load did not clean up: {name}")
        if load["selected_cpus"] != affinity:
            raise SystemExit(f"BitBake coexistence load did not use full affinity: {name}")
        if load["requested_workers"] != worker_count:
            raise SystemExit(f"BitBake coexistence worker count mismatch: {name}")
        if load["minimum_worker_cpu_percent"] < 15:
            raise SystemExit(f"BitBake coexistence load floor was not achieved: {name}")
for source in record["sources"].values():
    path = Path(source["path"])
    if hashlib.sha256(path.read_bytes()).hexdigest() != source["sha256"]:
        raise SystemExit(f"BitBake coexistence source digest mismatch: {path}")
policy = " ".join(record["policy"].values()).lower()
for required in ("read-only", "do not multiply", "never automatic", "neither root"):
    if required not in policy:
        raise SystemExit(f"BitBake coexistence policy is incomplete: {required}")
model = "\n".join(path.read_text(encoding="utf-8") for path in sorted(Path("crates/yoctui-model/src").rglob("*.rs")) if "tests" not in path.parts and path.name != "tests.rs")
cli = Path("crates/yoctui-cli/src/workspace_commands.rs").read_text(encoding="utf-8")
for required in (
    "bitbake_coexistence_diagnostic", "BB_NUMBER_THREADS", "PARALLEL_MAKE",
    "configured build parallelism can occupy every logical CPU",
):
    if required not in model:
        raise SystemExit(f"read-only coexistence diagnostic is missing: {required}")
if "coexistence policy: read-only; Yoctui changed no BitBake configuration" not in cli:
    raise SystemExit("inspect command does not disclose its read-only coexistence policy")
normal = record["scenarios"]["one_worker_per_logical_cpu"]["summary"]["median_p95_wake_latency_ms"]
over = record["scenarios"]["two_workers_per_logical_cpu"]["summary"]["median_p95_wake_latency_ms"]
print(f"BitBake coexistence audit valid: normal median p95 {normal:.4f} ms; 2x workers {over:.4f} ms")
PY

  python3 -m unittest scripts/test_cpu_saturation_harness.py scripts/test_measure_bitbake_coexistence.py
  cargo test -q -p yoctui-model coexistence_diagnostic
  cargo test -q -p yoctui-model parallel_make_parser
  cargo test -q -p yoctui --bin yoctui coexistence_output
  current="$(mktemp /tmp/yoctui-coexistence-current.XXXXXX.json)"
  trap 'unlink "$current" 2>/dev/null || true' RETURN
  ./scripts/measure-bitbake-coexistence.py \
    --revision "$(git rev-parse HEAD)" \
    --duration-seconds 1 \
    --repetitions 1 \
    --output "$current" >/dev/null
  python3 - "$current" <<'PY'
from pathlib import Path
import json
import sys

record = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
affinity = record["host"]["affinity_cpus"]
normal = record["scenarios"]["one_worker_per_logical_cpu"]
over = record["scenarios"]["two_workers_per_logical_cpu"]
if normal["workers"] != len(affinity) or over["workers"] != len(affinity) * 2:
    raise SystemExit("current coexistence measurement did not exercise both load levels")
if normal["summary"]["median_p95_wake_latency_ms"] > 100:
    raise SystemExit("current full-affinity coexistence path exceeds 100 ms")
if over["summary"]["median_p95_wake_latency_ms"] > 100:
    raise SystemExit("current oversubscribed coexistence path exceeds 100 ms")
print("current read-only coexistence diagnostic remains responsive at normal and 2x worker load")
PY
  trap - RETURN
  unlink "$current"
}

verify_real_poky() {
  python3 -m unittest scripts/test_capture_real_poky_performance.py
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess

root = Path("artifacts/performance/real-poky")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.real-poky-manifest.v1":
    raise SystemExit("real-Poky manifest schema is missing or unsupported")
revision = manifest.get("source_base_revision")
if not isinstance(revision, str) or len(revision) != 40:
    raise SystemExit("real-Poky source base must be an exact commit")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
artifact = root / manifest.get("artifact", "")
payload = artifact.read_bytes()
if hashlib.sha256(payload).hexdigest() != manifest.get("artifact_sha256"):
    raise SystemExit("real-Poky artifact digest mismatch")
record = json.loads(payload)
if record.get("schema") != "yoctui.performance.real-poky.v1":
    raise SystemExit("real-Poky evidence schema is unsupported")
if record.get("evidence_role") != "real_poky_build":
    raise SystemExit("fixture evidence cannot satisfy the real-Poky gate")
if record.get("source_base_revision") != revision:
    raise SystemExit("real-Poky source identity mismatch")
if record.get("binary") != manifest.get("binary"):
    raise SystemExit("real-Poky binary identity mismatch")
for name, expected in manifest.get("sources", {}).items():
    path = Path(name)
    if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
        raise SystemExit(f"real-Poky source digest mismatch: {name}")

measurement = record.get("measurement", {})
if measurement.get("clock") != "CLOCK_MONOTONIC":
    raise SystemExit("real-Poky evidence does not use monotonic time")
if measurement.get("warmup_seconds", 0) < 10 or measurement.get("window_seconds", 0) < 120:
    raise SystemExit("real-Poky warmup/window is too short")
if measurement.get("sustained_active_seconds") != measurement.get("window_seconds"):
    raise SystemExit("real-Poky job was not active for the complete measurement")
if measurement.get("sample_count", 0) < measurement.get("window_seconds", 0) - 1:
    raise SystemExit("real-Poky sample series is incomplete")
if measurement.get("statistic") != "10_percent_trimmed_mean":
    raise SystemExit("real-Poky robust statistic is absent")
if measurement.get("terminal") != {"columns": 160, "rows": 50}:
    raise SystemExit("real-Poky terminal dimensions differ from the contract")
trigger = measurement.get("workload_trigger", {})
if trigger.get("recipe") != "linux-yocto" or trigger.get("task") != "do_compile":
    raise SystemExit("real-Poky evidence did not start at the sustained kernel compile")

poky = record.get("poky", {})
if poky.get("release") != "6.0.2" or poky.get("target") != "linux-yocto":
    raise SystemExit("real-Poky evidence does not use the supported 6.0.2 kernel workload")
if poky.get("task") != "compile" or poky.get("force") is not False:
    raise SystemExit("real-Poky task identity is not the reviewed compile path")
if poky.get("preparation") != {
    "target": "linux-yocto", "task": "cleansstate", "through_daemon": True,
}:
    raise SystemExit("real-Poky preparation did not use the daemon-owned clean path")
if not poky.get("parallelism", {}).get("BB_NUMBER_THREADS") or not poky.get("parallelism", {}).get("PARALLEL_MAKE"):
    raise SystemExit("real-Poky BitBake parallelism is absent")
for name, repository in poky.get("repositories", {}).items():
    if len(repository.get("revision", "")) != 40 or "yocto-6.0.2" not in repository.get("describe", ""):
        raise SystemExit(f"real-Poky repository identity is invalid: {name}")
    if len(repository.get("working_diff_sha256", "")) != 64:
        raise SystemExit(f"real-Poky repository diff identity is absent: {name}")

summary = record.get("summary", {})
combined = summary.get("combined_cpu_trimmed_mean_percent_one_logical_cpu", 100.0)
if combined > 1.0:
    raise SystemExit(f"real-Poky combined Yoctui CPU exceeds 1.00%: {combined:.4f}%")
if summary.get("host_cpu_trimmed_mean_percent_total_capacity", 0.0) < 75.0:
    raise SystemExit("real-Poky workload did not saturate the reference host")
if summary.get("bitbake_cpu_trimmed_mean_percent_one_logical_cpu", 0.0) < 100.0:
    raise SystemExit("real-Poky BitBake process tree did not consume one logical CPU")
for role in ("daemon", "client"):
    process = summary.get(role, {})
    if process.get("rss_max_bytes", 0) <= 0 or process.get("threads_max", 0) <= 0:
        raise SystemExit(f"real-Poky {role} resource observations are absent")

responsiveness = record.get("responsiveness", {})
keys = responsiveness.get("key_to_visible_frame_samples_ms", [])
if len(keys) < 100 or responsiveness.get("key_to_visible_frame_p95_ms", 1000.0) > 100.0:
    raise SystemExit("real-Poky keyboard-to-frame latency exceeds the contract")
ipc = responsiveness.get("ipc_ms", {})
if ipc.get("build_command_to_ack_ms", 1000.0) > 100.0:
    raise SystemExit("real-Poky build-command acknowledgement exceeds 100 ms")
if ipc.get("cancellation_command_to_ack_ms", 1000.0) > 250.0:
    raise SystemExit("real-Poky cancellation acknowledgement exceeds 250 ms")
if ipc.get("fresh_attach_ms", 1000.0) > 100.0:
    raise SystemExit("real-Poky fresh attach exceeds 100 ms")
render = record.get("rendering", {})
if render.get("schema") != "yoctui.performance.render.v1":
    raise SystemExit("real-Poky render metrics are absent")
if not 0 < render.get("frames_per_second", 0) <= 6.0:
    raise SystemExit("real-Poky render cadence is stopped or excessive")
if render.get("coalesced", 0) <= 0:
    raise SystemExit("real-Poky render invalidations were not coalesced")

events = record.get("events", {})
if events.get("events", 0) <= 0 or events.get("by_type", {}).get("telemetry", 0) <= 0:
    raise SystemExit("real-Poky live daemon events were not observed")
if events.get("disconnects") != 0:
    raise SystemExit("real-Poky backend disconnected")
pressure = record.get("pressure", {})
if pressure.get("maximum_queue_depth", 257) > 256:
    raise SystemExit("real-Poky client queue exceeded its declared bound")
if pressure.get("reliable_waits", 1) != 0 or pressure.get("forced_resynchronizations", 1) != 0:
    raise SystemExit("real-Poky critical IPC delivery experienced pressure")
continuity = record.get("continuity", {})
if continuity.get("backend_disconnects") != 0 or continuity.get("client_reconnected") is not True:
    raise SystemExit("real-Poky backend/client continuity failed")
cancel = continuity.get("cancellation", {})
if not all(cancel.get(key) is True for key in ("requested", "acknowledged", "accepted")):
    raise SystemExit("real-Poky cancellation path is incomplete")
print(
    f"real-Poky performance valid: combined {combined:.4f}% of one logical CPU; "
    f"input p95 {responsiveness['key_to_visible_frame_p95_ms']:.3f} ms"
)
PY

  python3 -m py_compile scripts/capture-real-poky-performance.py
}
