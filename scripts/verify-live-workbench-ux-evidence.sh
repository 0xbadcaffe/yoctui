#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

./scripts/verify-next-generation-ui-evidence.sh

python3 - "$repo_root" <<'PY'
from __future__ import annotations

import datetime as dt
import json
import subprocess
import sys
import tomllib
from pathlib import Path

root = Path(sys.argv[1])
evidence = root / "artifacts/release-quality/next-generation-ui"
manifest = json.loads((evidence / "manifest.json").read_text(encoding="utf-8"))
rootfs = json.loads((evidence / "rootfs-evidence.json").read_text(encoding="utf-8"))

finished = dt.datetime.fromisoformat(manifest["finished_utc"].replace("Z", "+00:00"))
now = dt.datetime.now(dt.timezone.utc)
if finished > now or now - finished > dt.timedelta(days=90):
    raise SystemExit("live workbench UI evidence is stale or future-dated")
if subprocess.run(
    ["git", "merge-base", "--is-ancestor", manifest["source_commit"], "HEAD"],
    cwd=root,
    stdout=subprocess.DEVNULL,
    stderr=subprocess.DEVNULL,
).returncode:
    raise SystemExit("live workbench UI source is not an ancestor of HEAD")

required_scenarios = {
    "menus_and_availability",
    "build_completion",
    "image_manifest_pkgdata_rootfs",
    "context_terminal",
    "interactive_task_availability",
    "daemon_reconnect",
}
passed = {name for name, result in manifest["scenarios"].items() if result == "passed"}
if required_scenarios - passed:
    raise SystemExit("live workbench UI evidence lacks required M21 scenarios")
if rootfs["manifest_packages"] < 10 or rootfs["pkgdata_files"] < 1:
    raise SystemExit("live rootfs evidence lacks an authoritative package inventory")
if "build shell" not in (evidence / "terminal.txt").read_text(encoding="utf-8"):
    raise SystemExit("live workbench evidence lacks a daemon-owned context terminal")

identities = {}
for kind, role in (("latest", "latest_published_stable"), ("older", "older_supported")):
    path = root / f"docs/compatibility-evidence/{kind}.toml"
    record = tomllib.loads(path.read_text(encoding="utf-8"))
    observed = dt.date.fromisoformat(record["observed_at"])
    if observed > now.date() or (now.date() - observed).days > record["expires_after_days"]:
        raise SystemExit(f"{kind} supported-release evidence is stale or future-dated")
    if (
        record.get("evidence_level") != "live"
        or record.get("fixture_only") is not False
        or record.get("release_role") != role
    ):
        raise SystemExit(f"{kind} supported-release evidence is not a live claimed role")
    workflows = set(record.get("workflows", []))
    for workflow in ("core_build_events=passed", "build_cancellation=passed"):
        if workflow not in workflows:
            raise SystemExit(f"{kind} evidence lacks {workflow}")
    identities[kind] = f"{record['yocto_release']}/{record['bitbake_version']}"

print(
    "live workbench UX evidence verified: "
    f"UI {manifest['yocto_release']} at {manifest['source_commit'][:12]}; "
    f"latest {identities['latest']}; older {identities['older']}"
)
PY
