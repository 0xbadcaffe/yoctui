#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

python3 - <<'PY'
from pathlib import Path
import tomllib

required = {
    "M19-GOV-001",
    "FOUNDATION-UI-001", "FOUNDATION-UI-002", "FOUNDATION-UI-003",
    "NAV-UI-001",
    "TASKS-UI-001", "TASKS-UI-002", "TASKS-UI-003",
    "LOG-UI-001", "LOG-UI-002",
    "JOB-UI-001", "JOB-UI-002",
    "INSPECTOR-UI-001", "INSPECTOR-UI-002", "INSPECTOR-UI-003",
    "METRICS-MODEL-001", "METRICS-MODEL-002",
    "METRICS-UI-001", "METRICS-UI-002", "METRICS-UI-003",
    "METRICS-UI-004", "METRICS-UI-005", "METRICS-UI-006",
    "SYSTEM-UI-001", "SYSTEM-UI-002", "HEADER-UI-001",
    "FOOTER-UI-001", "FOOTER-UI-002",
    "SEARCH-UI-001", "PALETTE-UI-001", "DIALOG-UI-001",
    "MOUSE-UI-001", "A11Y-UI-001",
    "PERF-UI-001", "PERF-UI-002", "RESPONSIVE-UI-001",
    "VISUAL-TEST-001", "VISUAL-TEST-002", "VISUAL-TEST-003",
    "INPUT-TEST-001", "INPUT-TEST-002", "INPUT-TEST-003",
    "PTY-UI-TEST-001", "LIVE-UI-POKY-001", "README-UI-001",
    "UI-REGRESSION-001", "UI-CLEANUP-001", "M13-UI-001",
}

data = tomllib.loads(Path("docs/task-registry.toml").read_text(encoding="utf-8"))
tasks = {task["id"]: task for task in data.get("task", [])}
missing = sorted(required - tasks.keys())
if missing:
    raise SystemExit("missing next-generation UI tasks: " + ", ".join(missing))
wrong_milestone = sorted(task_id for task_id in required if tasks[task_id].get("milestone") != "M19")
if wrong_milestone:
    raise SystemExit("next-generation UI tasks outside M19: " + ", ".join(wrong_milestone))
incomplete = sorted(task_id for task_id in required if tasks[task_id].get("status") != "DONE")
if incomplete:
    raise SystemExit("next-generation UI tasks incomplete: " + ", ".join(incomplete))
print(f"next-generation UI registry complete: {len(required)} required tasks")
PY

# These commands remain explicit so the final gate independently exercises each
# acceptance category instead of trusting registry notes or one aggregate test.
cargo test --workspace --all-features
cargo test -p yoctui-ui semantic_snapshots
cargo test -p yoctui-ui target_design_golden
cargo test -p yoctui-ui style_invariants
cargo test -p yoctui-e2e next_generation_keymap
cargo test -p yoctui-ui breakpoint_matrix
cargo test -p yoctui-app next_generation_mouse
cargo test -p yoctui --test mouse_runtime next_generation_mouse
cargo test -p yoctui-e2e next_generation_pty
cargo test -p yoctui-ui accessibility_invariants
./scripts/test-next-generation-ui-performance.sh
./scripts/verify-next-generation-ui-evidence.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
./scripts/verify-roadmap.sh
./scripts/verify-product-complete.sh
