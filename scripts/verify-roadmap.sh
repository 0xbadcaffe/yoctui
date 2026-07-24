#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

required_files=(
  AGENTS.md
  docs/current-task.md
  docs/ui-spec.md
  docs/architecture.md
  docs/product-roadmap.md
  docs/implementation-status.md
  docs/task-registry.toml
)

for path in "${required_files[@]}"; do
  if [[ ! -s "$path" ]]; then
    printf 'missing or empty governance file: %s\n' "$path" >&2
    exit 1
  fi
done

python3 - <<'PY'
from pathlib import Path
import re
import sys
import tomllib

registry_path = Path("docs/task-registry.toml")
data = tomllib.loads(registry_path.read_text(encoding="utf-8"))
tasks = data.get("task", [])
valid = set(data.get("status_values", []))

if not tasks:
    raise SystemExit("task registry contains no tasks")

ids = [task.get("id") for task in tasks]
if any(not task_id for task_id in ids):
    raise SystemExit("every task must have an id")
if len(ids) != len(set(ids)):
    raise SystemExit("task ids must be unique")

by_id = {task["id"]: task for task in tasks}

for task in tasks:
    status = task.get("status")
    if status not in valid:
        raise SystemExit(f'{task["id"]}: invalid status {status!r}')
    if not isinstance(task.get("required"), bool):
        raise SystemExit(f'{task["id"]}: required must be boolean')
    if not task.get("verify"):
        raise SystemExit(f'{task["id"]}: missing verification commands')
    for dep in task.get("depends_on", []):
        if dep not in by_id:
            raise SystemExit(f'{task["id"]}: unknown dependency {dep}')
        if dep == task["id"]:
            raise SystemExit(f'{task["id"]}: self dependency')

# Cycle check.
visiting = set()
visited = set()

def visit(task_id):
    if task_id in visited:
        return
    if task_id in visiting:
        raise SystemExit(f"dependency cycle involving {task_id}")
    visiting.add(task_id)
    for dep in by_id[task_id].get("depends_on", []):
        visit(dep)
    visiting.remove(task_id)
    visited.add(task_id)

for task_id in by_id:
    visit(task_id)

current = Path("docs/current-task.md").read_text(encoding="utf-8")
match = re.search(r"\*\*ID:\*\*\s*([A-Z0-9-]+)", current)
if not match:
    raise SystemExit("docs/current-task.md must contain '**ID:** TASK-ID'")
current_id = match.group(1)
if current_id not in by_id:
    raise SystemExit(f"current task {current_id} is not in task registry")
if by_id[current_id]["status"] not in {"NOT_STARTED", "IN_PROGRESS", "BLOCKED"}:
    raise SystemExit(f"current task {current_id} is already DONE")

print(f"roadmap valid: {len(tasks)} tasks; current task {current_id}")
PY
