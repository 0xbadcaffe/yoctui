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
from html.parser import HTMLParser
from urllib.parse import parse_qs, urlsplit
import hashlib
import re
import struct
import subprocess
import tomllib

published_readme = Path("README.md").read_text(encoding="utf-8")
raw_base = "https://raw.githubusercontent.com/0xbadcaffe/yoctui/master/"
blob_base = "https://github.com/0xbadcaffe/yoctui/blob/master/"
# The registry must not resolve workspace-inherited links relative to the CLI
# crate directory. Validate published URLs before mapping them to local assets
# for the existing hash, order and anchor checks.
published_links = re.findall(r'(?:src|href)="([^"]+)"', published_readme)
published_links += re.findall(r'!?\[[^\]]+\]\(([^)\s]+)\)', published_readme)
for target in published_links:
    assert target.startswith("#") or urlsplit(target).scheme == "https", f"Registry-relative README link: {target}"
for source in re.findall(r'<img\b[^>]*\bsrc="([^"]+)"', published_readme):
    if "docs/media/" in source:
        assert source.startswith(raw_base), f"Screenshot must use direct raw image URL: {source}"
assert raw_base + "docs/media/yoctui-header.png" in published_readme
readme = published_readme.replace(raw_base, "").replace(blob_base, "")
def prose_headings(markdown, level="#{1,6}"):
    prose = re.sub(r"^```[^\n]*\n.*?^```[ \t]*$", "", markdown, flags=re.M | re.S)
    return re.findall(rf"^{level} (.+)$", prose, re.M)

assert prose_headings("# Title\n```bash\n# shell comment\n```\n", "#") == ["Title"]
assert prose_headings("# Title\n# Duplicate\n", "#") == ["Title", "Duplicate"]
assert prose_headings(readme, "#") == ["Yoctui"], "README title must not contain a version"
assert "Current source version:" not in readme, "Display the version in the crates.io badge only"
header = readme.split("<!-- /yoctui-header -->", 1)[0]
assert "<!-- yoctui-header -->" in header, "Missing branded README header"

class HeaderLinks(HTMLParser):
    def __init__(self):
        super().__init__()
        self.hrefs = set()
        self.images = {}

    def handle_starttag(self, tag, attrs):
        values = dict(attrs)
        if tag == "a":
            self.hrefs.add(values["href"])
        if tag == "img":
            assert values.get("alt", "").strip(), "Header image needs alternative text"
            self.images[values["src"]] = values

links = HeaderLinks()
links.feed(header)
for target in (
    "docs/operator-guide.md", "#install", "#features",
    "#quickstart-poky-build-environment", "docs/testing.md#completion-gate",
    "LICENSE", "https://github.com/0xbadcaffe/yoctui",
    "https://github.com/0xbadcaffe/yoctui/issues",
    "https://github.com/0xbadcaffe/yoctui/actions/workflows/ci.yml",
    "https://crates.io/crates/yoctui",
):
    assert target in links.hrefs, f"Missing header link: {target}"
    parsed = urlsplit(target)
    if not parsed.scheme:
        destination = Path(parsed.path or "README.md")
        assert destination.is_file(), f"Missing local header target: {target}"
        if parsed.fragment:
            headings = re.findall(r"^#{1,6} (.+)$", destination.read_text(), re.M)
            slugs = {re.sub(r"[^\w -]", "", title.lower()).replace(" ", "-") for title in headings}
            assert parsed.fragment in slugs, f"Missing header anchor: {target}"
assert any("/actions/workflows/ci.yml/badge.svg" in src for src in links.images)
version_badges = [urlsplit(src) for src in links.images if urlsplit(src).netloc == "img.shields.io" and urlsplit(src).path == "/crates/v/yoctui"]
assert len(version_badges) == 1, "Use one dynamic published-version badge"
badge_query = parse_qs(version_badges[0].query)
assert badge_query.get("cacheSeconds") == ["300"], "Limit published-version badge caching"
assert re.fullmatch(r"\d+\.\d+\.\d+", badge_query.get("release", [""])[0]), "Provide a release cache-refresh key"
assert any("rust-stable" in src for src in links.images)
assert "codecov" not in header.lower(), "No configured Codecov integration"
assert "discord" not in header.lower(), "No configured Discord invite"
assert "92%" not in header and "1.81+" not in header
banner = Path("docs/media/yoctui-header.png")
assert str(banner) in links.images, "Missing repository-owned banner"
png = banner.read_bytes()
assert png.startswith(b"\x89PNG\r\n\x1a\n"), "Header must be a PNG"
assert png[12:16] == b"IHDR"
width, height = struct.unpack(">II", png[16:24])
assert 2 <= width / height <= 3.5, "Header must stay compact and wide"
assert len(png) <= 2 * 1024 * 1024, "Header should remain under 2 MiB"
print("README header checks passed")

# Every main section must be reachable before readers scroll into the guide.
contents_start = "<!-- yoctui-contents -->"
contents_end = "<!-- /yoctui-contents -->"
assert readme.count(contents_start) == readme.count(contents_end) == 1
contents = readme.split(contents_start, 1)[1].split(contents_end, 1)[0]
assert readme.index(contents_end) < readme.index("\n## Install\n")
assert readme.index(contents_end) < readme.index('href="docs/media/screenshots/07-idle-dashboard.png"')

def heading_slug(title):
    return re.sub(r"[^\w -]", "", title.lower()).replace(" ", "-")

main_sections = prose_headings(readme, "##")
section_links = re.findall(r"\[[^\]]+\]\(#([^)]+)\)", contents)
assert len(section_links) == len(set(section_links)), "Duplicate contents link"
assert {heading_slug(title) for title in main_sections} <= set(section_links), "Link every main README section at the beginning"
assert "build-from-source" in section_links, "Expose optimized source build instructions"

# Validate all README-local Markdown targets, not just the branded header.
for target in re.findall(r"!?\[[^\]]+\]\(([^)\s]+)\)", readme):
    parsed = urlsplit(target)
    if parsed.scheme or parsed.netloc:
        continue
    destination = Path(parsed.path or "README.md")
    assert destination.is_file(), f"Missing README link target: {target}"
    if parsed.fragment:
        headings = re.findall(r"^#{1,6} (.+)$", destination.read_text(), re.M)
        assert parsed.fragment in {heading_slug(title) for title in headings}, f"Missing README link anchor: {target}"

# Syntax-check examples only: never install, start a daemon, or submit a build.
examples = re.findall(r"^```bash\n(.*?)^```", readme, re.M | re.S)
assert examples, "Missing Bash installation/quickstart examples"
for example in examples:
    syntax = subprocess.run(["bash", "-n"], input=example, text=True, capture_output=True)
    assert syntax.returncode == 0, syntax.stderr
for command in (
    "cargo install yoctui --locked -j 2",
    "cargo build --release --locked -p yoctui --bin yoctui -j 2",
    "cargo install --locked --path crates/yoctui-cli --force --bin yoctui -j 2",
    'export PATH="${CARGO_HOME:-$HOME/.cargo}/bin:$PATH"',
):
    assert command in readme, f"Missing verified installation/build instruction: {command}"
assert "may lag the source repository" in readme
assert "do not change BitBake/make parallelism" in readme
assert "boot/.debug" in readme, "Distinguish debugger symbols from stripped boot images"
assert "Readable text of any extension" in readme, "Document arbitrary-suffix Hardware text viewing"
assert "only TXT, PDF" not in readme, "Do not retain obsolete Hardware text limits"
assert "cargo build --locked -p yoctui\n" not in readme, "Interactive source build must use release mode"
print("README section navigation, local links and Bash example checks passed")

gallery_manifest = Path("docs/media/screenshots/manifest.toml")
assert gallery_manifest.is_file(), "Missing README screenshot provenance"
gallery = tomllib.loads(gallery_manifest.read_text(encoding="utf-8"))
artifacts = gallery.get("artifact", [])
expected_gallery_ids = (
    "active-build-tasks", "kernel-device-tree", "uboot-device-tree",
    "kernel-menuconfig", "uboot-menuconfig", "rootfs-composition",
    "idle-dashboard", "failed-build-errors", "editor-application-menu",
    "terminal-sessions", "device-tree-editor", "device-tree-compile-options",
    "cloning", "cancelling", "search-empty", "gitui-diff", "gitui-commit",
    "offline-dashboard", "saved-build-history", "saved-build-logs", "systemd-services", "system-dbus", "udev-rules",
)
assert tuple(item.get("id") for item in artifacts) == expected_gallery_ids
assert gallery.get("authority") == "production TestBackend cell/style goldens"

class GalleryImages(HTMLParser):
    def __init__(self):
        super().__init__()
        self.images = []

    def handle_starttag(self, tag, attrs):
        if tag != "img":
            return
        values = dict(attrs)
        source = values.get("src", "")
        if source.startswith("docs/media/screenshots/"):
            assert values.get("alt", "").strip(), f"Gallery image needs alt text: {source}"
            self.images.append(source)

gallery_images = GalleryImages()
gallery_images.feed(readme)
workflow_order = (
    "idle-dashboard", "cloning", "offline-dashboard", "active-build-tasks",
    "failed-build-errors", "cancelling", "search-empty", "saved-build-history",
    "saved-build-logs", "editor-application-menu", "terminal-sessions",
    "gitui-diff", "gitui-commit", "kernel-device-tree", "uboot-device-tree",
    "kernel-menuconfig", "uboot-menuconfig", "device-tree-editor",
    "device-tree-compile-options", "rootfs-composition", "systemd-services", "system-dbus", "udev-rules",
)
assert set(workflow_order) == set(expected_gallery_ids)
files_by_id = {item["id"]: item["file"] for item in artifacts}
expected_files = [files_by_id[key] for key in workflow_order]
assert gallery_images.images == expected_files, "README must show every screen once in workflow order"
for item in artifacts:
    image = Path(item["file"])
    source = Path(item["source"])
    assert image.is_file() and source.is_file(), f"Missing gallery input/output for {item['id']}"
    assert hashlib.sha256(image.read_bytes()).hexdigest() == item["sha256"]
    assert hashlib.sha256(source.read_bytes()).hexdigest() == item["source_sha256"]
    png = image.read_bytes()
    assert png.startswith(b"\x89PNG\r\n\x1a\n") and png[12:16] == b"IHDR"
    assert struct.unpack(">II", png[16:24]) == (1600, 1000)
assert "fixture values" in readme and "Recorded live capture" in readme
print("README screenshot gallery checks passed")

flamegraph = Path("artifacts/flamegraph/yoctui.svg")
flamegraph_summary = Path("artifacts/flamegraph/summary.txt")
assert flamegraph.is_file() and flamegraph.stat().st_size > 100_000
assert flamegraph_summary.is_file(), "Missing Flamegraph summary"
summary = dict(
    line.split("=", 1)
    for line in flamegraph_summary.read_text(encoding="utf-8").splitlines()
    if "=" in line and not line.startswith("dominant_")
)
assert summary.get("schema") == "yoctui.flamegraph.summary.v1"
assert int(summary["workload_frames"]) >= 1_000
assert int(summary["total_samples"]) >= 500
assert summary.get("unresolved_frames") == "0"
assert re.fullmatch(r"[0-9a-f]{16}", summary["workload_checksum"])
svg = flamegraph.read_text(encoding="utf-8")
readme_words = " ".join(readme.split())
assert "Yoctui workbench CPU profile" in svg
assert f'total_samples="{summary["total_event_count"]}"' in svg
assert "artifacts/flamegraph/yoctui.svg" in readme
assert "artifacts/flamegraph/summary.txt" in readme
assert f'{int(summary["workload_frames"]):,} frames' in readme_words
assert f'{int(summary["total_samples"]):,} real userspace samples' in readme_words
assert f'`{summary["workload_checksum"]}`' in readme
assert "v0.1.64" in readme and "September 6, 2026" in readme
assert "historical" in readme.lower()
print("README Flamegraph report checks passed")

required_sections = (
    "Screenshots",
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
    "Performance evidence",
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
