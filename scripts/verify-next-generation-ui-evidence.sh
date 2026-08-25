#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
evidence="${YOCTUI_NEXT_UI_EVIDENCE:-$repo_root/artifacts/release-quality/next-generation-ui}"
required=(
  manifest.json checksums.sha256 doctor.txt inspect.txt layers.txt recipes.txt
  build-status.log daemon.log failure-status.txt
  active-tasks.ansi active-tasks.txt active-tasks.meta
  completion.ansi completion.txt completion.meta
  failed-task.ansi failed-task.txt failed-task.meta
  terminal.ansi terminal.txt terminal.meta
  reconnect.ansi reconnect.txt reconnect.meta
)
for file in "${required[@]}"; do
  test -s "$evidence/$file" || { printf 'next-generation UI evidence missing: %s\n' "$file" >&2; exit 1; }
done

python3 - "$evidence" "$repo_root" <<'PY'
import json
import hashlib
import os
import re
import subprocess
import sys
from pathlib import Path

evidence = Path(sys.argv[1])
repo = Path(sys.argv[2])
manifest = json.loads((evidence / "manifest.json").read_text(encoding="utf-8"))
required_text = (
    "source_commit", "binary_sha256", "poky_revision", "poky_branch",
    "bitbake_version", "host", "machine", "distro", "yocto_release",
    "build_directory", "target", "started_utc", "finished_utc",
)
if manifest.get("schema") != 1 or manifest.get("label") != "live":
    raise SystemExit("next-generation UI manifest is not schema-1 live evidence")
for field in required_text:
    if not isinstance(manifest.get(field), str) or not manifest[field].strip():
        raise SystemExit(f"next-generation UI manifest field is missing: {field}")
if manifest["target"] != "core-image-minimal":
    raise SystemExit("next-generation UI evidence did not build core-image-minimal")
required_scenarios = {
    "startup", "environment", "recipes", "layers", "tasks", "live_logs",
    "build_completion", "safe_failure", "terminal", "daemon_reconnect",
}
scenarios = manifest.get("scenarios", {})
if required_scenarios - {name for name, value in scenarios.items() if value == "passed"}:
    raise SystemExit("next-generation UI manifest has incomplete live scenarios")
subprocess.run(
    ["git", "merge-base", "--is-ancestor", manifest["source_commit"], "HEAD"],
    cwd=repo,
    check=True,
)
binary = repo / "target" / "release" / "yoctui"
if not binary.is_file():
    raise SystemExit("next-generation UI release binary is unavailable")
actual_binary_sha256 = hashlib.sha256(binary.read_bytes()).hexdigest()
if actual_binary_sha256 != manifest["binary_sha256"]:
    raise SystemExit("next-generation UI evidence binary identity is stale")
if manifest["machine"] != "qemux86-64" or manifest["distro"] != "poky":
    raise SystemExit("next-generation UI evidence used an unexpected MACHINE or DISTRO")

for path in evidence.iterdir():
    if path.is_file() and path.stat().st_size > 2_000_000:
        raise SystemExit(f"next-generation UI evidence exceeds 2 MiB bound: {path.name}")
for name in ("active-tasks", "completion", "failed-task", "terminal", "reconnect"):
    raw = (evidence / f"{name}.ansi").read_bytes()
    if b"\x1b[?1049h" not in raw:
        raise SystemExit(f"{name} is not a real alternate-screen PTY capture")
    meta = (evidence / f"{name}.meta").read_text(encoding="utf-8")
    if "label=live" not in meta or "width=160" not in meta or "height=50" not in meta:
        raise SystemExit(f"{name} lacks live 160x50 terminal metadata")
    text = (evidence / f"{name}.txt").read_text(encoding="utf-8")
    if "Daemon: ✓ Connected" not in text or "Daemon: ✕ Disconnected" in text:
        raise SystemExit(f"{name} does not retain an attached daemon UI state")

active = (evidence / "active-tasks.txt").read_text(encoding="utf-8")
if "▶ Running" not in active or "Log Viewer" not in active:
    raise SystemExit("active Tasks evidence lacks a real running task/log presentation")
failed = (evidence / "failed-task.txt").read_text(encoding="utf-8")
if "Failed" not in failed:
    raise SystemExit("failed Tasks evidence lacks an exact failed state")

all_text = "\n".join(
    path.read_text(encoding="utf-8", errors="replace")
    for path in evidence.iterdir()
    if path.suffix in {".txt", ".log", ".json", ".meta"}
)
for anchor in ("qemux86-64", "poky", "core-image-minimal", "F1 Help"):
    if anchor not in all_text:
        raise SystemExit(f"next-generation UI evidence lacks anchor: {anchor}")
if not re.search(r"job .*Exited", (evidence / "build-status.log").read_text(encoding="utf-8")):
    raise SystemExit("live build did not retain an Exited daemon job")
if "Failed" not in (evidence / "failure-status.txt").read_text(encoding="utf-8"):
    raise SystemExit("safe failure evidence does not distinguish Failed")
PY

(cd "$evidence" && sha256sum -c checksums.sha256 >/dev/null)
printf 'next-generation UI live evidence verified: %s\n' "$evidence"
