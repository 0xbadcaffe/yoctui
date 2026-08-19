#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

bash -n scripts/test-release-compatibility.sh scripts/test-compatibility-matrix.sh

python3 <<'PY'
from pathlib import Path

root = Path.cwd()
workflow_path = root / ".github" / "workflows" / "ci.yml"
workflow = workflow_path.read_text(encoding="utf-8")
fast_script_path = root / "scripts" / "test-release-compatibility.sh"
matrix_script_path = root / "scripts" / "test-compatibility-matrix.sh"
fast_script = fast_script_path.read_text(encoding="utf-8")
matrix_script = matrix_script_path.read_text(encoding="utf-8")

for path in (fast_script_path, matrix_script_path):
    if not path.is_file() or path.stat().st_mode & 0o111 == 0:
        raise SystemExit(f"CI contract: required executable is missing: {path}")

required_workflow = (
    "push:",
    "pull_request:",
    "schedule:",
    "workflow_dispatch:",
    "compatibility-fast:",
    "./scripts/test-release-compatibility.sh",
    "compatibility-live:",
    "github.event_name == 'schedule' || github.event_name == 'workflow_dispatch'",
    "role: [older, latest]",
    "timeout-minutes: 45",
    "YOCTUI_LIVE_COMPATIBILITY: '1'",
    "./scripts/test-compatibility-matrix.sh --live ${{ matrix.role }}",
    "if: always()",
    "actions/upload-artifact@v4",
    "artifacts/compatibility/${{ matrix.role }}/",
    "if-no-files-found: error",
)
for value in required_workflow:
    if value not in workflow:
        raise SystemExit(f"CI contract: workflow lacks {value!r}")

live_job = workflow.split("  compatibility-live:\n", 1)[1]
if "pull_request" in live_job.split("    steps:\n", 1)[0]:
    raise SystemExit("CI contract: live compatibility job must not run on pull requests")

required_fast = (
    "yoctui-model",
    "yoctui-bitbake",
    "yoctui-app",
    "yoctui-ui",
    "compatibility_",
    "pytest bridge/tests",
    "test-compatibility-matrix.sh --evidence-only",
    "verify-live-compatibility.sh --evidence-only",
    "verify-compatibility.sh --structure-only",
)
for value in required_fast:
    if value not in fast_script:
        raise SystemExit(f"CI contract: deterministic gate lacks {value!r}")
for forbidden in ("git clone", "git fetch", "YOCTUI_LIVE_COMPATIBILITY=1", "--live latest", "--live older"):
    if forbidden in fast_script:
        raise SystemExit(f"CI contract: deterministic gate contains network/live operation {forbidden!r}")

for required in ("mktemp -d", "fetch_exact", "--depth 1", "runtime_dir", "state_dir", "artifact_dir"):
    if required not in matrix_script:
        raise SystemExit(f"CI contract: live matrix lacks isolation/bounds marker {required!r}")

print("compatibility CI contract valid")
PY
