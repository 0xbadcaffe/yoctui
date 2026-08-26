#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

python3 - <<'PY'
from pathlib import Path
import re
import tomllib

roadmap = Path("docs/workbench-ux-roadmap.md").read_text(encoding="utf-8")
ui_spec = Path("docs/ui-spec.md").read_text(encoding="utf-8")
architecture = Path("docs/architecture.md").read_text(encoding="utf-8")
product = Path("docs/product-roadmap.md").read_text(encoding="utf-8")
status = Path("docs/implementation-status.md").read_text(encoding="utf-8")
current = Path("docs/current-task.md").read_text(encoding="utf-8")
registry = tomllib.loads(Path("docs/task-registry.toml").read_text(encoding="utf-8"))

required_headings = {
    "Product outcome",
    "Non-negotiable constraints",
    "Research baseline",
    "Interaction architecture",
    "Built-in widget plan",
    "Third-party dependency and license gate",
    "Delivery phases and progress",
    "Test strategy",
    "Milestone completion definition",
}
headings = {
    match.group(1).strip()
    for match in re.finditer(r"^#{2,6}\s+(.+?)\s*$", roadmap, re.MULTILINE)
}
missing_headings = required_headings - headings
if missing_headings:
    raise SystemExit(f"workbench UX roadmap missing headings: {sorted(missing_headings)}")

builtins = {
    "Block", "Clear", "Paragraph", "List", "Table", "Tabs", "Scrollbar",
    "Gauge", "LineGauge", "Sparkline", "Chart", "BarChart", "Canvas", "Calendar",
}
missing_builtins = {name for name in builtins if f"`{name}`" not in roadmap}
if missing_builtins:
    raise SystemExit(f"workbench UX roadmap missing built-in widgets: {sorted(missing_builtins)}")

third_party = {
    "ratatui-image": "MIT",
    "ratatui-textarea": "MIT",
    "throbber-widgets-tui": "Zlib",
    "tui-big-text": "MIT OR Apache-2.0",
    "tui-checkbox": "MIT",
    "tui-logger": "MIT",
    "tui-menu": "MIT OR Apache-2.0",
    "tui-nodes": "MIT",
    "tui-piechart": "MIT",
    "tui-scrollview": "MIT OR Apache-2.0",
    "tui-term": "MIT",
    "tui-tree-widget": "MIT",
    "tui-widget-list": "MIT",
}
for crate, license_expression in third_party.items():
    row = next((line for line in roadmap.splitlines() if f"crates/{crate})" in line), None)
    if row is None:
        raise SystemExit(f"workbench UX roadmap missing third-party crate: {crate}")
    if f"| {license_expression} |" not in row:
        raise SystemExit(f"workbench UX roadmap has unexpected license for {crate}: {row}")

m21 = [task for task in registry.get("task", []) if task.get("milestone") == "M21"]
if len(m21) != 38:
    raise SystemExit(f"M21 must contain exactly 38 required tasks, found {len(m21)}")
if any(task.get("required") is not True for task in m21):
    raise SystemExit("every M21 task must be required")
done = [task for task in m21 if task.get("status") == "DONE"]
if not any(task["id"] == "UX-SPEC-001" and task["status"] == "DONE" for task in m21):
    raise SystemExit("UX-SPEC-001 must remain complete")

all_tasks = registry.get("task", [])
by_id = {task["id"]: task for task in all_tasks}
eligible = sorted(
    (
        task
        for task in m21
        if task["status"] != "DONE"
        and all(by_id[dependency]["status"] == "DONE" for dependency in task.get("depends_on", []))
    ),
    key=lambda task: task["priority"],
)
current_match = re.search(r"\*\*ID:\*\*\s*([A-Z0-9-]+)", current)
if not current_match:
    raise SystemExit("current task has no task ID")
current_id = current_match.group(1)
if eligible and current_id != eligible[0]["id"]:
    raise SystemExit(
        f"current M21 task must be highest-priority eligible {eligible[0]['id']}, found {current_id}"
    )

required_contracts = {
    "docs/ui-spec.md": (ui_spec, "## 33. One-stop workbench usability contract"),
    "docs/architecture.md": (architecture, "## M21 widget integration boundary"),
    "docs/product-roadmap.md": (product, "## M21 — One-Stop Yocto Workbench Usability"),
    "docs/implementation-status.md": (status, "M21 One-Stop Yocto Workbench Usability is active"),
}
for path, (text, marker) in required_contracts.items():
    if marker not in text:
        raise SystemExit(f"{path} missing M21 contract marker: {marker}")

done_count = len(done)
m21_percent = 100 * done_count / len(m21)
overall_done = sum(task.get("status") == "DONE" for task in all_tasks)
overall_percent = 100 * overall_done / len(all_tasks)
if f"**M21 total** | | | **{done_count}/{len(m21)} ({m21_percent:.1f}%)**" not in roadmap:
    raise SystemExit("roadmap M21 progress does not match registry")
if f"**{overall_done}/{len(all_tasks)} ({overall_percent:.1f}%)**" not in roadmap:
    raise SystemExit("roadmap overall progress does not match registry")

print(
    f"workbench UX roadmap valid: {len(m21)} M21 tasks; "
    f"{done_count} done; current {current_id}"
)
PY
