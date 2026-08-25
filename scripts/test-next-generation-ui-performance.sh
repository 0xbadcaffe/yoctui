#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

frames="${YOCTUI_UI_PERF_FRAMES:-500}"
max_ns_per_frame="${YOCTUI_UI_PERF_MAX_NS_PER_FRAME:-10000000}"
perf_target_dir="${YOCTUI_UI_PERF_TARGET_DIR:-$repo_root/target/ui-performance}"
if [[ ! "$frames" =~ ^[1-9][0-9]*$ ]]; then
  printf '%s\n' 'YOCTUI_UI_PERF_FRAMES must be a positive integer' >&2
  exit 2
fi
if [[ ! "$max_ns_per_frame" =~ ^[1-9][0-9]*$ ]]; then
  printf '%s\n' 'YOCTUI_UI_PERF_MAX_NS_PER_FRAME must be a positive integer' >&2
  exit 2
fi

mkdir -p artifacts/profile
work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT
summary="$work_dir/next-generation-ui.txt"

printf '%s\n' 'schema=yoctui.ui-performance.v1' >"$summary"
printf 'captured_utc=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >>"$summary"
printf 'terminal=160x48\nframes_per_scenario=%s\nmax_ns_per_frame=%s\n' \
  "$frames" "$max_ns_per_frame" >>"$summary"

for scenario in idle active-build large-metadata log-heavy telemetry; do
  output="$work_dir/$scenario.log"
  CARGO_TARGET_DIR="$perf_target_dir" \
    YOCTUI_PROFILE_FRAMES="$frames" YOCTUI_PROFILE_SCENARIO="$scenario" \
    cargo bench -q -p yoctui --bench workbench_profile 2>&1 | tee "$output"
  result="$(sed -n "s/^yoctui ui performance: scenario=$scenario frames=$frames checksum=\([0-9a-f]\{16\}\) elapsed_ms=\([0-9][0-9]*\) ns_per_frame=\([0-9][0-9]*\)$/\1 \2 \3/p" "$output")"
  if [[ -z "$result" ]]; then
    printf 'performance workload %s did not emit a valid completion record\n' "$scenario" >&2
    exit 1
  fi
  read -r checksum elapsed_ms ns_per_frame <<<"$result"
  if (( elapsed_ms == 0 || ns_per_frame > max_ns_per_frame )); then
    printf 'performance workload %s exceeded threshold: %s ns/frame > %s\n' \
      "$scenario" "$ns_per_frame" "$max_ns_per_frame" >&2
    exit 1
  fi
  key="${scenario//-/_}"
  printf '%s_checksum=%s\n%s_elapsed_ms=%s\n%s_ns_per_frame=%s\n' \
    "$key" "$checksum" "$key" "$elapsed_ms" "$key" "$ns_per_frame" >>"$summary"
done

mv "$summary" artifacts/profile/next-generation-ui.txt
printf '%s\n' 'next-generation UI performance thresholds passed'
