#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

python3 - "$repo_root" <<'PY'
from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path
from urllib.parse import unquote, urlsplit


root = Path(sys.argv[1]).resolve()
required_documents = {
    "README.md": {
        "Install",
        "Quickstart: Poky build environment",
    },
    "docs/operator-guide.md": {
        "Start a workspace safely",
        "Understand the persistent shell",
        "Daily image-build loop",
        "Core workspaces",
        "Browse and edit layers, recipes, and configuration",
        "Dependency, package, and signature evidence",
        "Image, SDK, QEMU, and Wic operations",
        "Testing, Security, and QA",
        "Maintenance",
        "Background jobs, cancellation, and terminal outcomes",
        "Settings, configuration, and sessions",
        "Troubleshooting",
        "Reference boundaries",
    },
    "docs/compatibility.md": {
        "Evidence levels",
        "Protocol and backend matrix",
        "Observed live Yocto combination",
        "Workflow compatibility matrix",
        "Host, runtime, and hardening matrix",
        "Adding a supported live combination",
    },
    "docs/testing.md": {"Testing", "Completion gate"},
    "docs/keymap.md": {
        "Yoctui Keymap Reference",
        "Global destinations",
        "Terminal prefix",
        "Customize safely",
    },
    "docs/rootfs-composition.md": {
        "Rootfs Composition Evidence",
        "Installed-package authority",
        "Logical-filesystem authority",
        "Recorded live boundary",
    },
    "docs/embedded-shell.md": {
        "Embedded Shells and Terminal Sessions",
        "Inherited Yocto shell",
        "Daemon-owned Terminal Sessions",
    },
    "docs/profiling.md": {"Profiling"},
    "docs/architecture.md": set(),
    "docs/protocol.md": set(),
    "docs/ui-spec.md": set(),
}

markdown_output = subprocess.check_output(
    ["git", "ls-files", "--", "*.md"], cwd=root, text=True
)
markdown_files = [root / line for line in markdown_output.splitlines() if line]
errors: list[str] = []
immutable_reference_sources = {
    root / "docs/reference/bitbake-cheatsheet-wrynose-6.0-bitbake-2.18.md",
}


def markdown_lines(path: Path) -> list[tuple[int, str]]:
    lines: list[tuple[int, str]] = []
    fenced = False
    fence = ""
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        stripped = line.lstrip()
        marker = stripped[:3]
        if marker in {"```", "~~~"}:
            if not fenced:
                fenced = True
                fence = marker
            elif marker == fence:
                fenced = False
                fence = ""
            continue
        if not fenced:
            lines.append((number, line))
    return lines


def heading_texts(path: Path) -> list[str]:
    headings = []
    for _, line in markdown_lines(path):
        match = re.match(r"^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$", line)
        if match:
            headings.append(match.group(1).strip())
    return headings


def slug_base(text: str) -> str:
    text = re.sub(r"<[^>]+>", "", text)
    text = re.sub(r"[`*_~]", "", text).lower().strip()
    text = "".join(
        character
        for character in text
        if character.isalnum() or character in {" ", "-", "_"}
    )
    return re.sub(r"\s+", "-", text).strip("-")


def anchors(path: Path) -> set[str]:
    counts: dict[str, int] = {}
    result: set[str] = set()
    for heading in heading_texts(path):
        base = slug_base(heading)
        index = counts.get(base, 0)
        result.add(base if index == 0 else f"{base}-{index}")
        counts[base] = index + 1
    text = path.read_text(encoding="utf-8")
    result.update(
        match.group(1)
        for match in re.finditer(
            r"<(?:a|span)\s+(?:[^>]*?\s)?id=[\"']([^\"']+)[\"'][^>]*>",
            text,
            re.IGNORECASE,
        )
    )
    return result


def destination_token(raw: str) -> str:
    value = raw.strip()
    if value.startswith("<") and ">" in value:
        return value[1 : value.index(">")]
    return value.split(maxsplit=1)[0] if value else ""


assert slug_base("Hello, World!") == "hello-world"
assert slug_base("SDK/QEMU & Wic") == "sdkqemu-wic"
assert destination_token('<docs/operator guide.md> "guide"') == "docs/operator guide.md"

for relative, required_headings in required_documents.items():
    path = root / relative
    if not path.is_file() or path.stat().st_size == 0:
        errors.append(f"required documentation file is missing or empty: {relative}")
        continue
    available = set(heading_texts(path))
    for heading in sorted(required_headings - available):
        errors.append(f"{relative}: missing required heading: {heading}")

anchor_cache: dict[Path, set[str]] = {}
inline_link = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
reference_link = re.compile(r"^\s*\[[^\]]+\]:\s*(\S+)")

for source in markdown_files:
    for line_number, line in markdown_lines(source):
        destinations = [match.group(1) for match in inline_link.finditer(line)]
        reference = reference_link.match(line)
        if reference:
            destinations.append(reference.group(1))
        for raw_destination in destinations:
            destination = destination_token(raw_destination)
            if not destination:
                errors.append(f"{source.relative_to(root)}:{line_number}: empty link target")
                continue
            parsed = urlsplit(destination)
            if parsed.scheme or parsed.netloc:
                continue
            local_path = unquote(parsed.path)
            fragment = unquote(parsed.fragment)
            if local_path.startswith("/"):
                target = root / local_path.lstrip("/")
            elif local_path:
                target = source.parent / local_path
            else:
                target = source
            target = target.resolve()
            try:
                target.relative_to(root)
            except ValueError:
                errors.append(
                    f"{source.relative_to(root)}:{line_number}: local link escapes repository: {destination}"
                )
                continue
            if not target.exists():
                errors.append(
                    f"{source.relative_to(root)}:{line_number}: missing local link target: {destination}"
                )
                continue
            if not target.is_file():
                errors.append(
                    f"{source.relative_to(root)}:{line_number}: local documentation link is not a file: {destination}"
                )
                continue
            if fragment:
                # Preserved third-party/reference snapshots may contain their own
                # renderer-specific hand-authored anchors. Keep validating that
                # project-authored links can reach the snapshot, but do not
                # rewrite or reinterpret its internal table of contents.
                if source in immutable_reference_sources:
                    continue
                if target.suffix.lower() != ".md":
                    errors.append(
                        f"{source.relative_to(root)}:{line_number}: fragment targets non-Markdown file: {destination}"
                    )
                    continue
                available_anchors = anchor_cache.setdefault(target, anchors(target))
                if fragment not in available_anchors:
                    errors.append(
                        f"{source.relative_to(root)}:{line_number}: missing Markdown anchor: {destination}"
                    )

if errors:
    for error in errors:
        print(f"documentation check: {error}", file=sys.stderr)
    raise SystemExit(1)

print(
    f"documentation links valid: {len(markdown_files)} tracked Markdown files",
    file=sys.stderr,
)
PY

for media in \
  docs/media/yoctui-demo.gif \
  docs/media/yoctui-live-active-tasks.svg \
  docs/media/yoctui-live-completion.svg \
  docs/media/yoctui-live-failed-task.svg \
  artifacts/flamegraph/yoctui.svg \
  artifacts/flamegraph/summary.txt
do
  if [[ ! -s "$media" ]]; then
    printf 'documentation check: visual artifact is missing or empty: %s\n' "$media" >&2
    exit 1
  fi
done

python3 scripts/render-next-generation-ui-screenshots.py --check
./scripts/render-m22-concept-screenshots.sh --check
python3 scripts/test-m22-concept-raster.py
python3 scripts/test-m22-live-design-gallery.py
./scripts/verify-live-m22-concept-evidence.sh
python3 scripts/test-live-m22-concept-evidence.py

cli_help="$(cargo run -q -p yoctui -- --help)"
for expected in \
  'A Ratatui frontend and control client for BitBake' \
  'Usage: yoctui' \
  '--backend <BACKEND>' \
  'inspect' \
  'recipes' \
  'layers' \
  'config' \
  'doctor'
do
  if [[ "$cli_help" != *"$expected"* ]]; then
    printf 'documentation check: CLI help is missing expected text: %s\n' "$expected" >&2
    exit 1
  fi
done

cargo build -q -p yoctui
headless_output="$(./scripts/headless-workload.sh target/debug/yoctui bridge)"
if [[ "$headless_output" != *"headless diagnostic completed"* ]]; then
  printf '%s\n' 'documentation check: isolated headless bridge diagnostic did not complete' >&2
  exit 1
fi

docs_config_dir="$(mktemp -d)"
trap 'rm -rf "$docs_config_dir"' EXIT
doctor_output="$(XDG_CONFIG_HOME="$docs_config_dir" cargo run -q -p yoctui -- doctor)"
if [[ "$doctor_output" != *"bridge protocol: ok"* ]]; then
  printf '%s\n' 'documentation check: isolated doctor did not validate the bridge protocol' >&2
  exit 1
fi
doctor_json="$(XDG_CONFIG_HOME="$docs_config_dir" cargo run -q -p yoctui -- doctor --json)"
python3 - "$doctor_json" <<'PY'
import json
import sys

report = json.loads(sys.argv[1])
if report.get("schema") != "yoctui.doctor.compatibility.v1":
    raise SystemExit("documentation check: Doctor JSON schema is missing")
if report.get("authority") not in {"current", "unavailable", "invalid"}:
    raise SystemExit("documentation check: Doctor JSON authority is invalid")
PY

while IFS= read -r script
do
  if ! bash -n "$script"; then
    printf 'documentation check: invalid shell syntax: %s\n' "$script" >&2
    exit 1
  fi
done < <(git ls-files -- '*.sh' | LC_ALL=C sort)

printf '%s\n' 'documentation checks passed'
