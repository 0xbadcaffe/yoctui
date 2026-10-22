#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

python3 - <<'PY'
from pathlib import Path
from html import unescape
from html.parser import HTMLParser
from urllib.parse import parse_qs, urlsplit
import hashlib
import json
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
    return [(unescape(html), unescape(link)) for html, link in
            re.findall(r'(?:src|href)="([^"]+)"|\]\(([^)\s]+)\)', markdown)]

assert prose_headings("# Title\n```bash\n# shell comment\n```\n", "#") == ["Title"]
assert prose_headings("# Title\n# Duplicate\n", "#") == ["Title", "Duplicate"]
assert prose_headings(readme, "#") == ["Yoctui"], "README title must not contain a version"
assert prose_headings(readme, "##") == [
    "Features", "Install", "Quickstart", "Documentation", "Contributing", "License",
]
assert links_in('<a href="https://example.org/?a=1&amp;b=2">') == [
    ("https://example.org/?a=1&b=2", ""),
]
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
assert "#license" in header_targets and blob_base + "LICENSE" in readme
for section in ("features", "install", "quickstart", "bitbake-setup", "daemon-and-systemd-service", "documentation", "contributing", "license"):
    assert "#" + section in header_targets, f"Missing README navigation: {section}"
ci_badges = [urlsplit(target) for target in header_targets if "/actions/workflows/ci.yml/badge.svg" in target]
assert len(ci_badges) == 1
ci_query = parse_qs(ci_badges[0].query)
source_version = tomllib.loads(Path("Cargo.toml").read_text())["workspace"]["package"]["version"]
assert ci_query.get("branch") == ["master"]
assert ci_query.get("event") == ["push"]
assert ci_query.get("refresh") == [source_version], "Refresh CI badge when releasing"
version_badges = [urlsplit(target) for target in header_targets if urlsplit(target).netloc == "img.shields.io" and urlsplit(target).path == "/crates/v/yoctui"]
assert len(version_badges) == 1
query = parse_qs(version_badges[0].query)
assert query.get("cacheSeconds") == ["300"]
assert query.get("release") == [source_version], "Refresh crates.io badge when releasing"
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
assert len(header_images.sources) == 9, "Keep the banner and eight status/info badges"
assert "Contributions are welcome" in readme and "MIT License" in readme
banner = Path("docs/media/yoctui-header.png")
assert raw_base + str(banner) in header_images.sources
png = banner.read_bytes()
assert png.startswith(b"\x89PNG\r\n\x1a\n") and png[12:16] == b"IHDR"
width, height = struct.unpack(">II", png[16:24])
assert 2 <= width / height <= 3.5 and len(png) <= 2 * 1024 * 1024
print("README header and documentation links passed")

# The main animation is a live capture, separate from the deterministic gallery.
animation = Path("docs/media/yoctui-poky-workflow.gif")
capture = json.loads(animation.with_suffix(".json").read_text())
gif = animation.read_bytes()
assert raw_base + str(animation) in readme
assert gif[:6] in (b"GIF87a", b"GIF89a")
assert struct.unpack("<HH", gif[6:10]) == (1600, 1000)
assert 8 <= capture["frame_count"] <= 500
assert len(gif) <= 10 * 1024 * 1024, "Keep README animation lightweight"
assert hashlib.sha256(gif).hexdigest() == capture["gif_sha256"]
assert len(gif) == capture["gif_bytes"]
assert set(capture["chapters"]) == {"dashboard", "tasks", "composition", "files"}
assert capture["authority"] == "native optimized Yoctui attached to a real daemon"
assert capture["time_compressed"] is True
assert re.fullmatch(r"[0-9a-f]{64}", capture["binary_sha256"])
assert all(re.fullmatch(r"[0-9a-f]{64}", frame["sha256"]) for frame in capture["source_frames"])
assert raw_base + "docs/media/screenshots/07-idle-dashboard.png" in readme
print("Live README animation and static fallback checks passed")

# Syntax only: these examples must never install, initialize, or launch work.
for name, markdown in {"README.md": readme, **manuals}.items():
    for example in re.findall(r"^```(?:bash|sh)\n(.*?)^```", markdown, re.M | re.S):
        syntax = subprocess.run(["bash", "-n"], input=example, text=True, capture_output=True)
        assert syntax.returncode == 0, f"{name}: {syntax.stderr}"
for command in (
    "sudo apt install -y build-essential pkg-config curl ca-certificates python3 git",
    "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh",
    "cargo install yoctui --locked -j 2",
    'export POKY_DIR="$HOME/src/poky"',
    'source "$POKY_DIR/oe-init-build-env" "$BUILDDIR"',
    "yoctui daemon status", "yoctui daemon start", "yoctui attach",
    "yoctui daemon service install", "systemctl --user enable --now yoctui.service",
    "yoctui daemon service status",
    'python3 -m venv "$HOME/.venvs/bitbake-setup"',
    'init --setup-dir-name yoctui-demo',
    'source "$HOME/bitbake-builds/yoctui-demo/build/init-build-env"',
):
    assert command in readme, f"Missing install/quickstart command: {command}"
assert "feature/bitbake-setup" not in readme, "Released instructions must link to master"
assert "not yet in a published release" not in readme
assert "`cargo install` does **not** install or enable a systemd service" in readme
assert "**Build Environment → w**" in readme
for instruction in ("`F8`", "`i` to choose an image", "`core-image-minimal`", "`b` to review the build"):
    assert instruction in readme, f"Missing image-build quickstart instruction: {instruction}"
user = manuals["docs/user-guide.md"]
development = manuals["docs/development.md"]
for command in ('export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"', 'export BUILDDIR="$POKY_DIR/build-yoctui"', 'source "$POKY_DIR/oe-init-build-env" "$BUILDDIR"', 'test -f "$POKY_DIR/oe-init-build-env"'):
    assert command in user, f"Missing workspace setup instruction: {command}"
for command in ("cargo build --release --locked -p yoctui --bin yoctui -j 2", "cargo install --locked --path crates/yoctui-cli --force --bin yoctui -j 2"):
    assert command in development, f"Missing optimized source installation: {command}"
assert "may lag the source repository" not in readme + development
assert readme.index('export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"') < readme.index("cargo install yoctui --locked -j 2")
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
    version_cells = "".join(f"1:{char}" for char in f"yoctui v{source_version}")
    assert version_cells in source.read_text(), f"Refresh screenshot version: {source}"
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
assert re.fullmatch(r"\d+\.\d+\.\d+", summary["application_version"])
assert f'v{summary["application_version"]}' in profile
assert summary["captured_utc"][:10] in profile
assert re.fullmatch(r"[0-9a-f]{40}", summary["source_revision"])
assert summary["runtime_source_dirty"] == "false"
assert summary["event_unit"] == "cycles"
assert re.fullmatch(r"[0-9a-f]{64}", summary["benchmark_sha256"])
receipt = json.loads(Path("docs/performance/renderer-hotspots-0.1.353.json").read_text())
assert receipt["schema"] == "yoctui.renderer-hotspots.v1"
comparison = receipt["final_comparison"]
assert len(comparison["results"]) == 10
for result in comparison["results"].values():
    assert re.fullmatch(r"[0-9a-f]{16}", result["checksum"])
    assert len(result["baseline_ns_per_frame"]) == comparison["paired_repeats"]
    assert len(result["candidate_ns_per_frame"]) == comparison["paired_repeats"]
if source_version == receipt["release_version"]:
    for name, digest in receipt["optimized_runtime_files_sha256"].items():
        assert hashlib.sha256(Path(name).read_bytes()).hexdigest() == digest, name
print("Manual Flamegraph report checks passed")
PY
printf 'README quickstart and relocated manual checks passed\n'
