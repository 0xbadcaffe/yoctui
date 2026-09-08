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
import tomllib

readme = Path("README.md").read_text(encoding="utf-8")
assert re.findall(r"^# (.+)$", readme, re.M) == ["Yoctui"], "README title must not contain a version"
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
gallery_manifest = Path("docs/media/screenshots/manifest.toml")
assert gallery_manifest.is_file(), "Missing README screenshot provenance"
gallery = tomllib.loads(gallery_manifest.read_text(encoding="utf-8"))
artifacts = gallery.get("artifact", [])
expected_gallery_ids = (
    "active-build-tasks", "kernel-device-tree", "uboot-device-tree",
    "kernel-menuconfig", "uboot-menuconfig", "rootfs-composition",
    "idle-dashboard", "failed-build-errors", "editor-application-menu",
    "terminal-sessions",
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
expected_files = [item["file"] for item in artifacts]
assert gallery_images.images == expected_files, "README gallery order differs from provenance"
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
