#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

python3 - <<'PY'
from pathlib import Path
from html.parser import HTMLParser
from urllib.parse import parse_qs, urlsplit
import hashlib
import re
import struct
import subprocess
import tomllib

raw_base = "https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/"
blob_base = "https://github.com/0xbadcaffe/yoctui/blob/master/"
readme = Path("README.md").read_text(encoding="utf-8")
manual_paths = (
    "docs/user-guide.md", "docs/keyboard-shortcuts.md", "docs/terminal-sessions.md",
    "docs/hardware-projects.md", "docs/kernel-and-firmware.md", "docs/rootfs-inspection.md",
    "docs/development.md", "docs/profiling.md", "docs/screenshots.md", "docs/README.md",
)
manuals = {name: Path(name).read_text(encoding="utf-8") for name in manual_paths}

def prose_headings(markdown, level="#{1,6}"):
    prose = re.sub(r"^```[^\n]*\n.*?^```[ \t]*$", "", markdown, flags=re.M | re.S)
    return re.findall(rf"^{level} (.+)$", prose, re.M)

def heading_slug(title):
    title = re.sub(r"[`*_~]", "", title).lower().strip()
    return re.sub(r"[^\w -]", "", title).replace(" ", "-")

def links_in(markdown):
    return re.findall(r'(?:src|href)="([^"]+)"|\]\(([^)\s]+)\)', markdown)

assert prose_headings("# Title\n```bash\n# shell comment\n```\n", "#") == ["Title"]
assert prose_headings("# Title\n# Duplicate\n", "#") == ["Title", "Duplicate"]
assert prose_headings(readme, "#") == ["Yoctui"], "README title must not contain a version"
assert prose_headings(readme, "##") == ["Install", "Quickstart", "Documentation"]
assert "Current source version:" not in readme, "Use the dynamic published-version badge"

# README URLs must work on crates.io as well as GitHub. Validate relocated manual
# links against their actual source directories, including published repository URLs.
for name, markdown in {"README.md": readme, **manuals}.items():
    for html_target, markdown_target in links_in(markdown):
        target = html_target or markdown_target
        parsed = urlsplit(target)
        if name == "README.md":
            assert target.startswith("#") or parsed.scheme == "https", f"Registry-relative README link: {target}"
        if target.startswith((raw_base, blob_base)):
            base = raw_base if target.startswith(raw_base) else blob_base
            local = urlsplit(target[len(base):])
            destination = Path(local.path)
            fragment = local.fragment
        elif not parsed.scheme and not parsed.netloc:
            destination = Path(name).parent / parsed.path if parsed.path else Path(name)
            fragment = parsed.fragment
        else:
            continue
        assert destination.is_file(), f"{name}: missing link target: {target}"
        if fragment:
            slugs = {heading_slug(title) for title in prose_headings(destination.read_text())}
            assert fragment in slugs, f"{name}: missing link anchor: {target}"

header = readme.split("<!-- /yoctui-header -->", 1)[0]
assert "<!-- yoctui-header -->" in header
header_targets = {html or markdown for html, markdown in links_in(header)}
assert "https://github.com/0xbadcaffe/yoctui/actions/workflows/ci.yml" in header_targets
assert "https://crates.io/crates/yoctui" in header_targets
assert blob_base + "LICENSE" in header_targets
assert any("/actions/workflows/ci.yml/badge.svg" in target for target in header_targets)
version_badges = [urlsplit(target) for target in header_targets if urlsplit(target).netloc == "img.shields.io" and urlsplit(target).path == "/crates/v/yoctui"]
assert len(version_badges) == 1
query = parse_qs(version_badges[0].query)
assert query.get("cacheSeconds") == ["300"]
assert re.fullmatch(r"\d+\.\d+\.\d+", query.get("release", [""])[0])
assert "codecov" not in header.lower() and "discord" not in header.lower()
assert "92%" not in header and "1.81+" not in header

class Images(HTMLParser):
    def __init__(self):
        super().__init__()
        self.sources = []

    def handle_starttag(self, tag, attrs):
        if tag == "img":
            values = dict(attrs)
            assert values.get("alt", "").strip(), "Image needs alternative text"
            self.sources.append(values["src"])

header_images = Images()
header_images.feed(header)
banner = Path("docs/media/yoctui-header.png")
assert raw_base + str(banner) in header_images.sources
png = banner.read_bytes()
assert png.startswith(b"\x89PNG\r\n\x1a\n") and png[12:16] == b"IHDR"
width, height = struct.unpack(">II", png[16:24])
assert 2 <= width / height <= 3.5 and len(png) <= 2 * 1024 * 1024
print("README header and documentation links passed")

# Syntax only: these examples must never install, initialize, or launch work.
for name, markdown in {"README.md": readme, **manuals}.items():
    for example in re.findall(r"^```(?:bash|sh)\n(.*?)^```", markdown, re.M | re.S):
        syntax = subprocess.run(["bash", "-n"], input=example, text=True, capture_output=True)
        assert syntax.returncode == 0, f"{name}: {syntax.stderr}"
for command in (
    "cargo install yoctui --locked -j 2",
    "yoctui daemon status", "yoctui daemon start", "yoctui --backend bridge",
):
    assert command in readme, f"Missing install/quickstart command: {command}"
user = manuals["docs/user-guide.md"]
development = manuals["docs/development.md"]
for command in ('export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"', 'export BUILDDIR="$POKY_DIR/build-yoctui"', 'source "$POKY_DIR/oe-init-build-env" "$BUILDDIR"', 'test -f "$POKY_DIR/oe-init-build-env"'):
    assert command in user, f"Missing workspace setup instruction: {command}"
for command in ("cargo build --release --locked -p yoctui --bin yoctui -j 2", "cargo install --locked --path crates/yoctui-cli --force --bin yoctui -j 2"):
    assert command in development, f"Missing optimized source installation: {command}"
assert "may lag the source repository" in development
assert "do not change BitBake/make parallelism" in development
assert "boot/.debug" in manuals["docs/kernel-and-firmware.md"]
assert "Readable text of any extension" in manuals["docs/hardware-projects.md"]
assert "only TXT, PDF" not in user
for destination in ("docs/user-guide.md", "docs/keyboard-shortcuts.md", "docs/README.md"):
    assert blob_base + destination in readme, f"Missing manual link: {destination}"
print("README and manual Bash examples passed")

# Keep every deterministic screenshot and its provenance checks after moving
# the full gallery out of README. Its reviewed workflow order is unchanged.
gallery = tomllib.loads(Path("docs/media/screenshots/manifest.toml").read_text(encoding="utf-8"))
artifacts = gallery.get("artifact", [])
expected_ids = (
    "active-build-tasks", "kernel-device-tree", "uboot-device-tree",
    "kernel-menuconfig", "uboot-menuconfig", "rootfs-composition",
    "idle-dashboard", "failed-build-errors", "editor-application-menu",
    "terminal-sessions", "device-tree-editor", "device-tree-compile-options",
    "cloning", "cancelling", "search-empty", "gitui-diff", "gitui-commit",
    "offline-dashboard", "saved-build-history", "saved-build-logs", "systemd-services", "system-dbus", "udev-rules",
)
assert tuple(item.get("id") for item in artifacts) == expected_ids
assert gallery.get("authority") == "production TestBackend cell/style goldens"
gallery_text = manuals["docs/screenshots.md"]
gallery_images = Images()
gallery_images.feed(gallery_text)
assert all(source.startswith(raw_base) for source in gallery_images.sources)
workflow_order = (
    "idle-dashboard", "cloning", "offline-dashboard", "active-build-tasks",
    "failed-build-errors", "cancelling", "search-empty", "saved-build-history",
    "saved-build-logs", "editor-application-menu", "terminal-sessions",
    "gitui-diff", "gitui-commit", "kernel-device-tree", "uboot-device-tree",
    "kernel-menuconfig", "uboot-menuconfig", "device-tree-editor",
    "device-tree-compile-options", "rootfs-composition", "systemd-services", "system-dbus", "udev-rules",
)
assert set(workflow_order) == set(expected_ids)
files_by_id = {item["id"]: item["file"] for item in artifacts}
assert gallery_images.sources == [raw_base + files_by_id[key] for key in workflow_order]
for item in artifacts:
    image, source = Path(item["file"]), Path(item["source"])
    assert image.is_file() and source.is_file()
    png = image.read_bytes()
    assert hashlib.sha256(png).hexdigest() == item["sha256"]
    assert hashlib.sha256(source.read_bytes()).hexdigest() == item["source_sha256"]
    assert png.startswith(b"\x89PNG\r\n\x1a\n") and png[12:16] == b"IHDR"
    assert struct.unpack(">II", png[16:24]) == (1600, 1000)
assert "fixture values" in gallery_text and "Raster provenance" in gallery_text
print("Manual screenshot gallery checks passed")

flamegraph = Path("artifacts/flamegraph/yoctui.svg")
assert flamegraph.is_file() and flamegraph.stat().st_size > 100_000
summary = dict(line.split("=", 1) for line in Path("artifacts/flamegraph/summary.txt").read_text().splitlines() if "=" in line and not line.startswith("dominant_"))
assert summary.get("schema") == "yoctui.flamegraph.summary.v1"
assert int(summary["workload_frames"]) >= 1_000 and int(summary["total_samples"]) >= 500
assert summary.get("unresolved_frames") == "0"
assert re.fullmatch(r"[0-9a-f]{16}", summary["workload_checksum"])
svg = flamegraph.read_text()
assert "Yoctui workbench CPU profile" in svg
assert f'total_samples="{summary["total_event_count"]}"' in svg
profile = manuals["docs/profiling.md"]
words = " ".join(profile.split())
assert "[Profiling](profiling.md)" in manuals["docs/README.md"]
assert "artifacts/flamegraph/yoctui.svg" in profile and "artifacts/flamegraph/summary.txt" in profile
assert f'{int(summary["workload_frames"]):,} frames' in words
assert f'{int(summary["total_samples"]):,} real userspace samples' in words
assert f'`{summary["workload_checksum"]}`' in profile
assert "v0.1.64" in profile and "September 6, 2026" in profile and "historical" in profile
print("Manual Flamegraph report checks passed")
PY
printf 'README quickstart and relocated manual checks passed\n'
