#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

scenario=""
duration=20
binary="$repo_root/target/release/yoctui"
call_graph="${YOCTUI_PERF_CALL_GRAPH:-lbr}"
maximum_dropped_ppm="${YOCTUI_PERF_MAX_DROPPED_PPM:-5000}"
revision="$(git rev-parse HEAD)"
pids=()
while (($#)); do
  case "$1" in
    --scenario) scenario="$2"; shift 2 ;;
    --duration) duration="$2"; shift 2 ;;
    --binary) binary="$2"; shift 2 ;;
    --revision) revision="$2"; shift 2 ;;
    --pid) pids+=("$2"); shift 2 ;;
    *) printf 'unknown argument: %s\n' "$1" >&2; exit 2 ;;
  esac
done
[[ -n "$scenario" ]] || { printf '%s\n' '--scenario is required' >&2; exit 2; }
((${#pids[@]} > 0)) || { printf '%s\n' 'at least one --pid ROLE=PID is required' >&2; exit 2; }
[[ "$duration" =~ ^[0-9]+$ ]] && ((duration >= 10)) || {
  printf '%s\n' '--duration must be an integer of at least 10 seconds' >&2
  exit 2
}
[[ -x "$binary" ]] || { printf 'binary is not executable: %s\n' "$binary" >&2; exit 2; }
[[ "$revision" =~ ^[0-9a-f]{40}$ ]] || { printf '%s\n' '--revision must be a full commit' >&2; exit 2; }
git merge-base --is-ancestor "$revision" HEAD || {
  printf 'revision is not an ancestor of HEAD: %s\n' "$revision" >&2
  exit 2
}
command -v perf >/dev/null || { printf '%s\n' 'perf is required' >&2; exit 2; }
command -v flamegraph >/dev/null || { printf '%s\n' 'flamegraph is required' >&2; exit 2; }

pid_numbers=()
for value in "${pids[@]}"; do
  [[ "$value" =~ ^[a-zA-Z][a-zA-Z0-9_-]*=[1-9][0-9]*$ ]] || {
    printf 'invalid --pid value: %s\n' "$value" >&2
    exit 2
  }
  pid="${value#*=}"
  [[ -r "/proc/$pid/stat" ]] || { printf 'PID is unavailable: %s\n' "$pid" >&2; exit 2; }
  pid_numbers+=("$pid")
done
pid_csv="$(IFS=,; printf '%s' "${pid_numbers[*]}")"

artifact_dir="$repo_root/artifacts/performance/profiles"
raw_root="${YOCTUI_PROFILE_RAW_DIR:-$repo_root/target/performance-profiles}"
mkdir -p "$artifact_dir" "$raw_root"
raw_dir="$(mktemp -d "$raw_root/$scenario.XXXXXX")"
perf_data="$raw_dir/$scenario.perf.data"
perf_log="$raw_dir/$scenario.perf.log"
filter_report="$raw_dir/$scenario.filter.txt"
flat_report="$artifact_dir/$scenario.perf.txt"
svg="$artifact_dir/$scenario.svg"
summary="$artifact_dir/$scenario.json"
processes_json="$raw_dir/$scenario.processes.json"

python3 - "$processes_json" "${pids[@]}" <<'PY'
from pathlib import Path
import json
import sys

processes = []
for value in sys.argv[2:]:
    role, raw_pid = value.split("=", 1)
    pid = int(raw_pid)
    command = (
        Path(f"/proc/{pid}/cmdline").read_bytes().rstrip(b"\0")
        .replace(b"\0", b" ").decode("utf-8", errors="replace")
    )
    processes.append({
        "role": role,
        "pid": pid,
        "executable": str(Path(f"/proc/{pid}/exe").resolve()),
        "command": command,
    })
Path(sys.argv[1]).write_text(json.dumps(processes, indent=2) + "\n", encoding="utf-8")
PY

perf record --no-buildid-mmap -F 499 -e cycles:u \
  --call-graph "$call_graph" -p "$pid_csv" -o "$perf_data" -- sleep "$duration" \
  2>&1 | tee "$perf_log"
perf report --stdio -i "$perf_data" --no-children -g none --percent-limit 0.1 \
  > "$flat_report"
YOCTUI_FLAMEGRAPH_FILTER_REPORT="$filter_report" \
YOCTUI_FLAMEGRAPH_MAX_DROPPED_PPM="$maximum_dropped_ppm" \
  flamegraph --perfdata "$perf_data" --deterministic --min-width 0.05 \
  --post-process 'python3 scripts/filter-flamegraph-stacks.py' \
  --title "Yoctui $scenario pre-optimization" --output "$svg"

python3 scripts/summarize-runtime-profile.py \
  --scenario "$scenario" \
  --revision "$revision" \
  --duration-seconds "$duration" \
  --call-graph "$call_graph" \
  --maximum-dropped-ppm "$maximum_dropped_ppm" \
  --binary "$binary" \
  --perf-log "$perf_log" \
  --flat-report "$flat_report" \
  --filter-report "$filter_report" \
  --svg "$svg" \
  --processes-json "$processes_json" \
  --output "$summary" \
