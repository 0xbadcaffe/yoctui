#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

python3 - <<'PY'
from pathlib import Path
import re
import tomllib

roadmap = Path("docs/widgets-and-dependencies.md").read_text(encoding="utf-8")
ui_spec = Path("docs/interface-behavior.md").read_text(encoding="utf-8")
architecture = Path("docs/architecture.md").read_text(encoding="utf-8")
registry = tomllib.loads(Path("docs/design/acceptance-contracts.toml").read_text(encoding="utf-8"))

required_headings = {
    "Product outcome",
    "Non-negotiable constraints",
    "Interaction architecture",
    "Built-in widget plan",
    "Third-party dependency and license gate",
    "Test strategy",
    "Acceptance criteria",
}
headings = {
    match.group(1).strip()
    for match in re.finditer(r"^#{2,6}\s+(.+?)\s*$", roadmap, re.MULTILINE)
}
missing_headings = required_headings - headings
if missing_headings:
    raise SystemExit(f"workbench design missing headings: {sorted(missing_headings)}")

builtins = {
    "Block", "Clear", "Paragraph", "List", "Table", "Tabs", "Scrollbar",
    "Gauge", "LineGauge", "Sparkline", "Chart", "BarChart", "Canvas", "Calendar",
}
missing_builtins = {name for name in builtins if f"`{name}`" not in roadmap}
if missing_builtins:
    raise SystemExit(f"workbench design missing built-in widgets: {sorted(missing_builtins)}")

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
        raise SystemExit(f"workbench design missing third-party crate: {crate}")
    if f"| {license_expression} |" not in row:
        raise SystemExit(f"workbench design has unexpected license for {crate}: {row}")

m21 = [task for task in registry.get("task", []) if task.get("milestone") == "M21"]
if len(m21) != 38:
    raise SystemExit(f"M21 must contain exactly 38 required tasks, found {len(m21)}")
if any(task.get("required") is not True for task in m21):
    raise SystemExit("every M21 task must be required")
if not any(task["id"] == "UX-SPEC-001" and task["status"] == "DONE" for task in m21):
    raise SystemExit("UX-SPEC-001 must remain complete")

required_contracts = {
    "docs/interface-behavior.md": (ui_spec, "## Workbench usability contract"),
    "docs/architecture.md": (architecture, "## Widget integration boundary"),
}
for path, (text, marker) in required_contracts.items():
    if marker not in text:
        raise SystemExit(f"{path} missing M21 contract marker: {marker}")

print("workbench design valid: widget, license, UI and architecture contracts")
PY
