#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

./scripts/verify-design-contracts.sh

python3 - <<'PY'
from pathlib import Path
import tomllib

data = tomllib.loads(Path("docs/design/acceptance-contracts.toml").read_text(encoding="utf-8"))
incomplete = [
    task for task in data.get("task", [])
    if task.get("required") and task.get("status") != "DONE"
]

if incomplete:
    print("required product acceptance remains incomplete:")
    for task in sorted(incomplete, key=lambda t: t["id"]):
        print(f'  {task["id"]}: {task["status"]} — {task["title"]}')
    raise SystemExit(1)

print("all required product acceptance entries are verified")
PY
