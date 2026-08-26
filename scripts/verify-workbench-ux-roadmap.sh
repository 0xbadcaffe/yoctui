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
if len(m21) != 37:
    raise SystemExit(f"M21 must contain exactly 37 required tasks, found {len(m21)}")
if any(task.get("required") is not True for task in m21):
    raise SystemExit("every M21 task must be required")
done = [task for task in m21 if task.get("status") == "DONE"]
if [task["id"] for task in done] != ["UX-SPEC-001"]:
    raise SystemExit(f"initial M21 progress must be UX-SPEC-001 only, found {done}")
if "**ID:** UX-LICENSE-001" not in current or "**Status:** NOT_STARTED" not in current:
    raise SystemExit("current task must be the not-started M21 license gate")

required_contracts = {
    "docs/ui-spec.md": (ui_spec, "## 33. One-stop workbench usability contract"),
    "docs/architecture.md": (architecture, "## M21 widget integration boundary"),
    "docs/product-roadmap.md": (product, "## M21 — One-Stop Yocto Workbench Usability"),
    "docs/implementation-status.md": (status, "M21 One-Stop Yocto Workbench Usability is active"),
}
for path, (text, marker) in required_contracts.items():
    if marker not in text:
        raise SystemExit(f"{path} missing M21 contract marker: {marker}")

if "**M21 total** | | | **1/37 (2.7%)**" not in roadmap:
    raise SystemExit("roadmap M21 progress does not match registry")
if "**541/577 (93.8%)**" not in roadmap:
    raise SystemExit("roadmap overall progress does not match registry")

print("workbench UX roadmap valid: 37 M21 tasks; 1 done; current UX-LICENSE-001")
PY
