#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

./scripts/verify-roadmap.sh

python3 - <<'PY'
from pathlib import Path
import tomllib

data = tomllib.loads(Path("docs/task-registry.toml").read_text(encoding="utf-8"))
incomplete = [
    task for task in data.get("task", [])
    if task.get("required") and task.get("status") != "DONE"
]

if incomplete:
    print("required product tasks remain incomplete:")
    for task in sorted(incomplete, key=lambda t: (t.get("priority", 9999), t["id"])):
        print(f'  {task["id"]}: {task["status"]} — {task["title"]}')
    raise SystemExit(1)

print("all required product tasks are DONE")
PY
