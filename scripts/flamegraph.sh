#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
command -v cargo-flamegraph >/dev/null || { printf '%s\n' 'cargo-flamegraph is required; install it with cargo install flamegraph' >&2; exit 2; }
command -v perf >/dev/null || { printf '%s\n' 'Linux perf is required; install the matching linux-perf package' >&2; exit 2; }
perf_probe="$(mktemp)"
if ! perf record --no-buildid-mmap -e dummy:u -o "$perf_probe" -- true >/dev/null 2>&1; then
  rm -f "$perf_probe"
  printf '%s\n' 'perf sampling is unavailable; grant CAP_PERFMON or lower kernel.perf_event_paranoid for this verification' >&2
  exit 2
fi
rm -f "$perf_probe"
mkdir -p artifacts/flamegraph
flamegraph_work_dir="$(mktemp -d)"
trap 'rm -rf "$flamegraph_work_dir"' EXIT
svg="$flamegraph_work_dir/yoctui.svg"
workload_log="$flamegraph_work_dir/workload.log"
summary="$flamegraph_work_dir/summary.txt"
filter_report="$flamegraph_work_dir/filter.txt"

YOCTUI_FLAMEGRAPH_FILTER_REPORT="$filter_report" \
YOCTUI_PROFILE_SCENARIO=large-metadata \
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C force-frame-pointers=yes" \
cargo flamegraph \
  -p yoctui \
  --deterministic \
  --cmd 'record -F 499 -e cycles:u --call-graph dwarf,8192 -g' \
  --min-width 0.01 \
  --post-process 'python3 scripts/filter-flamegraph-stacks.py' \
  --title 'Yoctui workbench CPU profile' \
  --subtitle 'Measured worst-case 4096-recipe / 1024-layer 160x48 rendering workload' \
  --output "$svg" \
  --bench workbench_profile \
  2>&1 | tee "$workload_log"

python3 scripts/validate-flamegraph.py "$svg" "$workload_log" "$filter_report" "$summary"
mv "$svg" artifacts/flamegraph/yoctui.svg
mv "$summary" artifacts/flamegraph/summary.txt
