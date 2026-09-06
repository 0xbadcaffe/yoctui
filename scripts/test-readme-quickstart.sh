#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
test -s README.md
grep -Fq 'oe-init-build-env' README.md
grep -Fq 'export BUILDDIR="$POKY_DIR/build-yoctui"' README.md
grep -Fq 'source "$POKY_DIR/oe-init-build-env" "$BUILDDIR"' README.md
grep -Fq 'yoctui --backend bridge' README.md
python3 - <<'PY'
from pathlib import Path

readme = Path("README.md").read_text(encoding="utf-8")
required_sections = (
    "Features",
    "Install",
    "Quickstart: Poky build environment",
    "Navigation",
    "Build, logs and errors",
    "Search source code and generated content",
    "Edit recipes and develop a patch",
    "Inspect an image, its packages and rootfs",
    "Boot with QEMU or connect over SSH",
    "Kernel, firmware and build analysis",
    "SDK, Wic, tests, security and maintenance",
    "Daemon and remote use",
    "Settings and team profiles",
    "Compatibility and troubleshooting",
    "Development and license",
)
for section in required_sections:
    assert f"\n## {section}\n" in readme, f"Missing operator section: {section}"
for command in (
    "cargo install yoctui --locked",
    "cargo install --locked --path crates/yoctui-cli --force",
    "yoctui daemon start",
    "yoctui daemon status",
    "yoctui attach",
):
    assert command in readme, f"Missing installation/startup command: {command}"
for widget in ("tui-logger", "tui-term", "tui-piechart", "tui-tree-widget"):
    assert widget in readme, f"Missing widget feature attribution: {widget}"
assert "[MIT-licensed](LICENSE)" in readme
assert "THIRD_PARTY_NOTICES.md" in readme
print("README operator coverage checks passed")
PY
echo "README quickstart command checks passed"
