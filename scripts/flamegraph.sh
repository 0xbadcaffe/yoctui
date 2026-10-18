#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"
profile_target_dir="${YOCTUI_FLAMEGRAPH_BUILD_TARGET_DIR:-${YOCTUI_PROFILE_TARGET_DIR:-$repo_root/target/ui-performance}}"
command -v flamegraph >/dev/null || { printf '%s\n' 'flamegraph is required; install it with cargo install flamegraph' >&2; exit 2; }
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
build_report="$flamegraph_work_dir/build.jsonl"
revision="$(git rev-parse HEAD)"
runtime_dirty=false
git diff --quiet HEAD -- Cargo.toml Cargo.lock crates || runtime_dirty=true

CARGO_TARGET_DIR="$profile_target_dir" \
RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-C force-frame-pointers=yes" \
cargo bench --locked -p yoctui-e2e --bench workbench_profile --no-run \
  --message-format=json >"$build_report"
benchmark="$(python3 - "$build_report" <<'PY'
import json
from pathlib import Path
import sys

messages = (json.loads(line) for line in Path(sys.argv[1]).read_text().splitlines())
binaries = [m["executable"] for m in messages if m.get("reason") == "compiler-artifact"
            and m.get("executable") and m["target"]["name"] == "workbench_profile"]
if len(binaries) != 1:
    raise SystemExit("Expected one renderer benchmark executable")
print(binaries[0])
PY
)"

# Keep perf.data and its temporary backups out of the checkout. Sampling starts
# only after Cargo exits, and the exact sampled binary is recorded below.
(
  cd "$flamegraph_work_dir"
  YOCTUI_FLAMEGRAPH_FILTER_REPORT="$filter_report" \
  YOCTUI_PROFILE_SCENARIO=large-metadata \
  flamegraph \
  --deterministic \
  --cmd 'record -F 499 -e cycles:u --call-graph fp -g' \
  --min-width 0.01 \
  --post-process "python3 $repo_root/scripts/filter-flamegraph-stacks.py" \
  --title 'Yoctui workbench CPU profile' \
  --subtitle 'CPU-cycle weights; 4096 recipes / 1024 layers; 160x48 fixture' \
  --output "$svg" -- "$benchmark"
) \
  2>&1 | tee "$workload_log"

python3 scripts/validate-flamegraph.py "$svg" "$workload_log" "$filter_report" "$summary"
{
  printf 'captured_utc=%s\nsource_revision=%s\nruntime_source_dirty=%s\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$revision" "$runtime_dirty"
  printf 'application_version=%s\n' "$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml)"
  printf 'host=%s\nkernel=%s\nrustc=%s\n' "$(hostname)" "$(uname -srmo)" "$(rustc --version)"
  printf 'workload_scenario=large-metadata\nterminal=160x48\nevent_unit=cycles\nsampling_hz=499\ncall_graph=fp\n'
  printf 'benchmark_sha256=%s\n' "$(sha256sum "$benchmark" | cut -d ' ' -f 1)"
  printf 'workload_source_sha256=%s\n' "$(sha256sum crates/yoctui-cli/benches/workbench_profile.rs | cut -d ' ' -f 1)"
} >>"$summary"
raw_dir="$profile_target_dir/flamegraph"
mkdir -p "$raw_dir"
cp "$flamegraph_work_dir/perf.data" "$raw_dir/perf.data"
cp "$workload_log" "$filter_report" "$raw_dir/"
perf report --no-inline --stdio -g none --no-children --sort symbol \
  -i "$raw_dir/perf.data" >"$raw_dir/self-time.txt"
mv "$svg" artifacts/flamegraph/yoctui.svg
mv "$summary" artifacts/flamegraph/summary.txt
