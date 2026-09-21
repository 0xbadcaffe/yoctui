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

verify_affinity() {
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess

root = Path("artifacts/performance/affinity")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.affinity-manifest.v1":
    raise SystemExit("affinity manifest schema is missing or unsupported")
revision = manifest.get("source_base_revision")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
artifact = root / manifest["artifact"]
if hashlib.sha256(artifact.read_bytes()).hexdigest() != manifest["artifact_sha256"]:
    raise SystemExit("affinity evidence digest mismatch")
record = json.loads(artifact.read_text(encoding="utf-8"))
if record.get("schema") != "yoctui.performance.affinity-audit.v1":
    raise SystemExit("affinity evidence schema is unsupported")
if record.get("revision") != revision:
    raise SystemExit("affinity evidence source identity mismatch")
if record["configuration"].get("repetitions") != 3:
    raise SystemExit("affinity evidence must contain three repeated trials")
allowed = record["host"]["affinity_cpus"]
reserved = record["host"]["reserved_logical_cpu"]
if len(allowed) < 2 or reserved not in allowed:
    raise SystemExit("affinity host identity is incomplete")
expected = {
    "shared_full_affinity": (allowed, allowed),
    "pinned_competing_cpu": (allowed, [reserved]),
    "pinned_reserved_cpu": (allowed[:-1], [reserved]),
}
for name, (load_cpus, probe_cpus) in expected.items():
    scenario = record["scenarios"].get(name)
    if scenario is None or len(scenario["trials"]) != 3:
        raise SystemExit(f"affinity scenario is incomplete: {name}")
    if scenario["load_cpus"] != load_cpus or scenario["probe_cpus"] != probe_cpus:
        raise SystemExit(f"affinity scenario CPU identity mismatch: {name}")
    if scenario["summary"]["median_p95_wake_latency_ms"] > 100:
        raise SystemExit(f"affinity scenario exceeded responsiveness bound: {name}")
    for load in scenario["saturation"]:
        if load["status"] != "completed" or load["children_reaped"] is not True:
            raise SystemExit(f"affinity load did not clean up: {name}")
        if load["selected_cpus"] != load_cpus:
            raise SystemExit(f"affinity load CPU set mismatch: {name}")
        if load["minimum_worker_cpu_percent"] < 25:
            raise SystemExit(f"affinity load was below its declared minimum: {name}")
decision = " ".join(manifest.get("decision", {}).values()).lower()
if "retain the inherited full affinity set" not in decision:
    raise SystemExit("affinity guidance no longer preserves the default path")
if "never select a logical cpu" not in decision:
    raise SystemExit("affinity guidance permits a hardcoded CPU")
baseline = record["scenarios"]["shared_full_affinity"]["summary"]["median_p95_wake_latency_ms"]
reserved_latency = record["scenarios"]["pinned_reserved_cpu"]["summary"]["median_p95_wake_latency_ms"]
print(
    "affinity audit valid: full set median p95 "
    f"{baseline:.4f} ms; one logical CPU reserved {reserved_latency:.4f} ms"
)
PY

  python3 -m unittest scripts/test_measure_affinity.py
  ./scripts/verify-saturation-responsiveness.sh --harness
  current="$(mktemp /tmp/yoctui-affinity-current.XXXXXX.json)"
  trap 'unlink "$current" 2>/dev/null || true' RETURN
  ./scripts/measure-affinity.py \
    --revision "$(git rev-parse HEAD)" \
    --duration-seconds 1 \
    --repetitions 1 \
    --output "$current" >/dev/null
  python3 - "$current" <<'PY'
from pathlib import Path
import json
import sys

record = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
allowed = record["host"]["affinity_cpus"]
full = record["scenarios"]["shared_full_affinity"]
if full["load_cpus"] != allowed or full["probe_cpus"] != allowed:
    raise SystemExit("current required affinity scenario reserved a CPU")
if full["summary"]["median_p95_wake_latency_ms"] > 100:
    raise SystemExit("current full-affinity scheduler latency exceeds 100 ms")
print("current full-affinity path remains responsive without a reserved CPU")
PY
  trap - RETURN
  unlink "$current"
}
