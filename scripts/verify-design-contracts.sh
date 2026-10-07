#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
python3 - <<'PYCODE'
from pathlib import Path
import tomllib

required = (
    "docs/development.md", "docs/ui-spec.md", "docs/architecture.md",
    "docs/workbench-design.md", "docs/product-roadmap.md",
    "docs/design/acceptance-contracts.toml",
)
for name in required:
    path = Path(name)
    if not path.is_file() or not path.stat().st_size:
        raise SystemExit(f"missing or empty development contract: {name}")
data = tomllib.loads(Path(required[-1]).read_text(encoding="utf-8"))
entries = data.get("task", [])
if not entries:
    raise SystemExit("acceptance contracts are empty")
ids = [entry.get("id") for entry in entries]
if not all(ids) or len(ids) != len(set(ids)):
    raise SystemExit("acceptance IDs must be nonempty and unique")
by_id = {entry["id"]: entry for entry in entries}
valid = set(data.get("status_values", []))
visiting, visited = set(), set()
def visit(identifier):
    if identifier in visiting:
        raise SystemExit(f"acceptance dependency cycle: {identifier}")
    if identifier in visited:
        return
    visiting.add(identifier)
    entry = by_id[identifier]
    if entry.get("status") not in valid or not isinstance(entry.get("required"), bool):
        raise SystemExit(f"invalid acceptance metadata: {identifier}")
    for dependency in entry.get("depends_on", []):
        if dependency not in by_id:
            raise SystemExit(f"unknown acceptance dependency: {dependency}")
        visit(dependency)
    visiting.remove(identifier)
    visited.add(identifier)
for identifier in ids:
    visit(identifier)
print(f"development/design contracts valid: {len(entries)} acceptance entries")
PYCODE
