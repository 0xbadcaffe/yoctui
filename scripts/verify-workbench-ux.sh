#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

python3 - <<'PY'
from pathlib import Path
import tomllib

data = tomllib.loads(Path("docs/task-registry.toml").read_text(encoding="utf-8"))
tasks = {task["id"]: task for task in data.get("task", [])}
m21 = {task_id: task for task_id, task in tasks.items() if task.get("milestone") == "M21"}
parent_id = "UX-001"

if len(m21) != 38 or parent_id not in m21:
    raise SystemExit(
        f"M21 must contain 37 children and {parent_id}; found {len(m21)} tasks"
    )

children = {task_id: task for task_id, task in m21.items() if task_id != parent_id}
if len(children) != 37:
    raise SystemExit(f"M21 must contain exactly 37 child tasks; found {len(children)}")
if any(task.get("required") is not True for task in m21.values()):
    raise SystemExit("every M21 task must remain required")

incomplete = sorted(
    task_id for task_id, task in children.items() if task.get("status") != "DONE"
)
if incomplete:
    raise SystemExit("M21 child tasks incomplete: " + ", ".join(incomplete))

declared = set(m21[parent_id].get("depends_on", []))
missing_dependencies = sorted(set(children) - declared)
unexpected_dependencies = sorted(declared - set(children))
if missing_dependencies or unexpected_dependencies:
    raise SystemExit(
        "UX-001 dependency set does not exactly match M21 children: "
        f"missing={missing_dependencies}, unexpected={unexpected_dependencies}"
    )

print("one-stop workbench registry ready: 37/37 child tasks DONE")
PY

# Exercise each release-quality category independently. Focused task tests are
# retained in the registry; the workspace test run covers them together while
# the real-PTY, performance, dependency, live, and documentation gates retain
# their environment-specific assertions.
./scripts/verify-workbench-ux-roadmap.sh
./scripts/verify-m21-concept-pack.py
./scripts/verify-widget-dependencies.sh
./scripts/verify-third-party-notices.sh

cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings

./scripts/test-workbench-ux-keymap.sh
./scripts/test-workbench-terminal.sh
./scripts/test-tui-snapshots.sh
./scripts/verify-ui-spec.sh
./scripts/test-workbench-ux-performance.sh
./scripts/test-flamegraph.sh

./scripts/test-live-workbench-ux.sh
./scripts/verify-live-workbench-ux-evidence.sh
./scripts/verify-compatibility.sh
./scripts/check-docs.sh
./scripts/verify-roadmap.sh

printf 'one-stop Yocto workbench verification passed\n'
