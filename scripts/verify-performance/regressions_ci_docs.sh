verify_regressions() {
  python3 -m unittest scripts/test_build_performance_regression_record.py
  python3 - <<'PY'
from pathlib import Path
import hashlib
import json
import subprocess
import tempfile

root = Path("artifacts/performance/regression")
manifest = json.loads((root / "manifest.json").read_text(encoding="utf-8"))
if manifest.get("schema") != "yoctui.performance.regression-manifest.v1":
    raise SystemExit("performance regression manifest schema is unsupported")
revision = manifest.get("source_base_revision")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", revision, "HEAD"],
    check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
)
for source, expected in manifest.get("sources", {}).items():
    if hashlib.sha256(Path(source).read_bytes()).hexdigest() != expected:
        raise SystemExit(f"performance regression source digest mismatch: {source}")
artifact = root / manifest.get("artifact", "")
if hashlib.sha256(artifact.read_bytes()).hexdigest() != manifest.get("artifact_sha256"):
    raise SystemExit("performance regression artifact digest mismatch")
with tempfile.TemporaryDirectory(prefix="yoctui-regression-") as directory:
    regenerated = Path(directory) / "metrics.json"
    subprocess.run(
        ["./scripts/build-performance-regression-record.py", "--output", str(regenerated)],
        check=True,
    )
    if regenerated.read_bytes() != artifact.read_bytes():
        raise SystemExit("performance regression record does not reproduce from retained evidence")
record = json.loads(artifact.read_text(encoding="utf-8"))
if record.get("schema") != "yoctui.performance.regression-record.v1":
    raise SystemExit("performance regression record schema is unsupported")
if set(record.get("metrics", {})) != {"cpu", "latency", "wakeups", "render", "pressure", "memory"}:
    raise SystemExit("performance regression metric coverage is incomplete")
hard = [
    metric
    for group in record["metrics"].values()
    for metric in group.values()
    if "hard_maximum" in metric
]
if len(hard) < 20 or not all(metric.get("passed") is True for metric in hard):
    raise SystemExit("one or more controlled performance hard gates failed")
if not all(record.get("correctness", {}).values()):
    raise SystemExit("one or more performance correctness checks failed")
if record.get("result") != {
    "hard_metrics": len(hard),
    "hard_metrics_passed": len(hard),
    "correctness_checks": len(record["correctness"]),
    "correctness_checks_passed": len(record["correctness"]),
    "passed": True,
}:
    raise SystemExit("performance regression aggregate result is inconsistent")
policy = " ".join(manifest.get("policy", {}).values()).lower()
if "hard failures" not in policy or "tiny uncontrolled variance" not in policy:
    raise SystemExit("performance regression variance policy is incomplete")
combined = record["metrics"]["cpu"]["real_poky_combined"]["value"]
print(
    f"performance regression record valid: {len(hard)} hard metrics and "
    f"{len(record['correctness'])} correctness checks; real-Poky CPU {combined:.4f}%"
)
PY
}

verify_ci() {
  bash -n scripts/verify-performance-ci-fast.sh
  python3 -m unittest scripts/test_performance_ci_contract.py
  ./scripts/verify-performance-ci-fast.sh
}

verify_docs() {
  python3 -m unittest scripts/test_performance_documentation.py
  ./scripts/check-docs.sh
  printf '%s\n' 'low-overhead architecture and tuning documentation valid'
}
