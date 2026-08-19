#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
command -v valgrind >/dev/null || { printf '%s\n' 'valgrind is required; install it before profiling' >&2; exit 2; }
mkdir -p artifacts/valgrind
profile_binary="$(
  cargo build -p yoctui --bench workbench_profile --message-format=json | \
    python3 -c 'import json, sys
for line in sys.stdin:
    event = json.loads(line)
    target = event.get("target", {})
    if event.get("reason") == "compiler-artifact" and target.get("name") == "workbench_profile" and event.get("executable"):
        print(event["executable"])
'
)"
if [[ ! -x "$profile_binary" ]]; then
  printf '%s\n' 'Valgrind workbench benchmark executable was not produced' >&2
  exit 1
fi
YOCTUI_PROFILE_FRAMES=128 \
valgrind --tool=memcheck --leak-check=full --show-leak-kinds=all --track-fds=yes --track-origins=yes --xml=yes --xml-file=artifacts/valgrind/report.xml "$profile_binary" >artifacts/valgrind/workload.txt 2>&1
if ! grep -Eq '^yoctui workbench profile: frames=128 checksum=[0-9a-f]{16} elapsed_ms=[1-9][0-9]*$' artifacts/valgrind/workload.txt; then
  printf '%s\n' 'Valgrind workbench benchmark did not complete' >&2
  exit 1
fi
python3 - <<'PY' | tee artifacts/valgrind/summary.txt
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

# Valgrind 3.25 can emit the literal diagnostic token `<unknown>` inside a
# `<what>` element without XML escaping it. Normalize only that known token;
# malformed or truncated reports still fail `ET.fromstring`.
report = Path("artifacts/valgrind/report.xml").read_text(encoding="utf-8")
root = ET.fromstring(report.replace("<unknown>", "&lt;unknown&gt;"))
leaks = root.find("leak_summary")
values = {
    name: int(leaks.findtext(f"{name}/bytes", "0").replace(",", ""))
    for name in ("definitely_lost", "indirectly_lost", "possibly_lost", "still_reachable")
}
kinds = [element.text for element in root.findall("error/kind")]
allowed = {"FdNotClosed", "Leak_StillReachable"}
fatal = [kind for kind in kinds if kind not in allowed]
fd_errors = [element for element in root.findall("error") if element.findtext("kind") == "FdNotClosed"]
unexpected_fds = [
    element.findtext("fd", "unknown")
    for element in fd_errors
    if not any(
        "tokio::signal::" in (frame.findtext("fn") or "")
        for frame in element.findall("stack/frame")
    )
]
print("Valgrind production workbench workload summary")
for name, value in values.items():
    print(f"{name}: {value} bytes")
print(f"Tokio signal descriptors reported: {len(fd_errors) - len(unexpected_fds)}")
if values["definitely_lost"] or values["indirectly_lost"] or fatal or unexpected_fds:
    print(f"fatal Memcheck findings: kinds={fatal} unexpected_fds={unexpected_fds}", file=sys.stderr)
    sys.exit(1)
PY
