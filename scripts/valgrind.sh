#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
command -v valgrind >/dev/null || { printf '%s\n' 'valgrind is required; install it before profiling' >&2; exit 2; }
mkdir -p artifacts/valgrind
cargo build -p yoctui
valgrind --tool=memcheck --leak-check=full --show-leak-kinds=all --track-fds=yes --track-origins=yes --xml=yes --xml-file=artifacts/valgrind/report.xml target/debug/yoctui --headless --backend bridge --build-dir "$repo_root" >artifacts/valgrind/workload.txt 2>&1
python3 - <<'PY' | tee artifacts/valgrind/summary.txt
import sys
import xml.etree.ElementTree as ET

root = ET.parse("artifacts/valgrind/report.xml").getroot()
leaks = root.find("leak_summary")
values = {
    name: int(leaks.findtext(f"{name}/bytes", "0").replace(",", ""))
    for name in ("definitely_lost", "indirectly_lost", "possibly_lost", "still_reachable")
}
kinds = [element.text for element in root.findall("error/kind")]
allowed = {"FdNotClosed", "Leak_StillReachable"}
fatal = [kind for kind in kinds if kind not in allowed]
print("Valgrind bridge workload summary")
for name, value in values.items():
    print(f"{name}: {value} bytes")
print(f"open descriptors reported: {kinds.count('FdNotClosed')}")
if values["definitely_lost"] or values["indirectly_lost"] or fatal:
    print(f"fatal Memcheck findings: {fatal}", file=sys.stderr)
    sys.exit(1)
PY
