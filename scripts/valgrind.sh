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
python3 -m unittest scripts/test_valgrind_policy.py
for frames in 16 128; do
  report="artifacts/valgrind/report.xml"
  workload="artifacts/valgrind/workload.txt"
  if [[ "$frames" == 16 ]]; then
    report="artifacts/valgrind/baseline-16.xml"
    workload="artifacts/valgrind/baseline-16-workload.txt"
  fi
  YOCTUI_PROFILE_FRAMES="$frames" \
    valgrind --tool=memcheck --leak-check=full --show-leak-kinds=all \
      --track-fds=yes --track-origins=yes --xml=yes --xml-file="$report" \
      "$profile_binary" >"$workload" 2>&1
  if ! grep -Eq "^yoctui workbench profile: frames=$frames checksum=[0-9a-f]{16} elapsed_ms=[1-9][0-9]*$" "$workload"; then
    printf 'Valgrind workbench benchmark did not complete: %s frames\n' "$frames" >&2
    exit 1
  fi
done
python3 scripts/check_valgrind.py artifacts/valgrind/report.xml \
  --baseline artifacts/valgrind/baseline-16.xml | tee artifacts/valgrind/summary.txt
