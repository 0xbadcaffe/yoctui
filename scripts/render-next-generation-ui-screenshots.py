#!/usr/bin/env python3
"""Render deterministic README SVGs from verified live semantic captures."""

from __future__ import annotations

import argparse
import html
import json
from pathlib import Path


SCENARIOS = {
    "active-tasks": ("Live task and log", "#7dd3fc"),
    "completion": ("Completed core-image-minimal build", "#86efac"),
    "failed-task": ("Typed safe-failure state", "#fca5a5"),
}


def render(capture: str, title: str, accent: str, manifest: dict[str, object]) -> str:
    lines = capture.rstrip("\n").splitlines()
    line_height = 15
    terminal_y = 96
    height = terminal_y + max(1, len(lines)) * line_height + 24
    source = str(manifest["source_commit"])
    poky = str(manifest["poky_revision"])
    binary = str(manifest["binary_sha256"])
    provenance = (
        f"source {source[:12]}  •  Poky {poky[:12]} ({manifest['poky_branch']})"
        f"  •  {manifest['bitbake_version']}  •  {manifest['target']}"
    )
    metadata = html.escape(json.dumps(manifest, sort_keys=True, separators=(",", ":")))
    rows = "\n".join(
        f'  <text x="18" y="{terminal_y + index * line_height}" '
        f'class="terminal">{html.escape(line)}</text>'
        for index, line in enumerate(lines)
    )
    return f"""<svg xmlns="http://www.w3.org/2000/svg" width="1180" height="{height}" viewBox="0 0 1180 {height}" role="img" aria-labelledby="title description">
<title id="title">Yoctui — {html.escape(title)}</title>
<desc id="description">Real Poky terminal capture from the verified next-generation UI evidence bundle.</desc>
<metadata>{metadata}</metadata>
<style>
  .heading {{ fill: #e2e8f0; font: 700 18px system-ui, sans-serif; }}
  .provenance {{ fill: #94a3b8; font: 12px system-ui, sans-serif; }}
  .hash {{ fill: #64748b; font: 11px ui-monospace, monospace; }}
  .terminal {{ fill: #dbeafe; font: 10.5px 'DejaVu Sans Mono', 'Liberation Mono', monospace; white-space: pre; }}
</style>
<rect width="1180" height="{height}" rx="12" fill="#020617"/>
<rect x="1" y="1" width="1178" height="{height - 2}" rx="11" fill="none" stroke="#334155"/>
<circle cx="20" cy="22" r="5" fill="#fb7185"/><circle cx="38" cy="22" r="5" fill="#fbbf24"/><circle cx="56" cy="22" r="5" fill="#4ade80"/>
<text x="78" y="29" class="heading">{html.escape(title)}</text>
<rect x="18" y="45" width="5" height="23" rx="2" fill="{accent}"/>
<text x="32" y="55" class="provenance">{html.escape(provenance)}</text>
<text x="32" y="72" class="hash">binary sha256 {html.escape(binary)}</text>
<line x1="18" y1="84" x2="1162" y2="84" stroke="#1e293b"/>
{rows}
</svg>
"""


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--evidence",
        type=Path,
        default=Path("artifacts/release-quality/next-generation-ui"),
    )
    parser.add_argument("--output", type=Path, default=Path("docs/media"))
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()

    manifest = json.loads((args.evidence / "manifest.json").read_text(encoding="utf-8"))
    stale: list[Path] = []
    for scenario, (title, accent) in SCENARIOS.items():
        capture = (args.evidence / f"{scenario}.txt").read_text(encoding="utf-8")
        expected = render(capture, title, accent, manifest)
        destination = args.output / f"yoctui-live-{scenario}.svg"
        if args.check:
            if (
                not destination.is_file()
                or destination.read_text(encoding="utf-8") != expected
            ):
                stale.append(destination)
        else:
            destination.parent.mkdir(parents=True, exist_ok=True)
            destination.write_text(expected, encoding="utf-8")

    if stale:
        names = ", ".join(str(path) for path in stale)
        raise SystemExit(f"live UI screenshots are missing or stale: {names}")
    action = "verified" if args.check else "rendered"
    print(f"live UI screenshots {action}: {len(SCENARIOS)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
