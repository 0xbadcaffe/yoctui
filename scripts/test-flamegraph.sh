#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

work_dir="$(mktemp -d)"
trap 'rm -rf "$work_dir"' EXIT
perf_target_dir="${YOCTUI_FLAMEGRAPH_TARGET_DIR:-$repo_root/target/ui-performance}"
CARGO_TARGET_DIR="$perf_target_dir" YOCTUI_PROFILE_FRAMES=128 \
  cargo bench -q -p yoctui --bench workbench_profile >"$work_dir/workload.log"
grep -Eq '^yoctui workbench profile: frames=128 checksum=[0-9a-f]{16} elapsed_ms=[1-9][0-9]*$' "$work_dir/workload.log"

python3 - "$work_dir" <<'PY'
from pathlib import Path
import sys

root = Path(sys.argv[1])
valid = """<svg><svg id="frames" total_samples="1000">
<g><title>workbench_profile (1,000 samples, 100%)</title></g>
<g><title>yoctui_ui::render_at (900 samples, 90%)</title></g></svg></svg>"""
(root / "valid.svg").write_text(valid, encoding="utf-8")
(root / "invalid.svg").write_text(valid.replace("yoctui_ui::render_at", "[unknown]"), encoding="utf-8")
(root / "profile.log").write_text(
    "yoctui workbench profile: frames=1000 checksum=0123456789abcdef elapsed_ms=1\n"
    "[ perf record: Captured and wrote 1.000 MB perf.data (1,000 samples) ]\n",
    encoding="utf-8",
)
(root / "filter.txt").write_text(
    "schema=yoctui.flamegraph.filter.v1\n"
    "raw_stack_lines=1\naccepted_stack_lines=1\nunresolved_stack_lines=0\n"
    "raw_event_count=1000\ndropped_unresolved_event_count=0\n"
    "dropped_unresolved_ppm=0\n",
    encoding="utf-8",
)
(root / "folded.txt").write_text(
    "workbench_profile;yoctui_ui::render_at 9999\n"
    "workbench_profile;[unknown] 1\n",
    encoding="utf-8",
)
PY
YOCTUI_FLAMEGRAPH_FILTER_REPORT="$work_dir/generated-filter.txt" \
  python3 scripts/filter-flamegraph-stacks.py \
  <"$work_dir/folded.txt" >"$work_dir/filtered.txt"
if grep -Fq '[unknown]' "$work_dir/filtered.txt"; then
  printf '%s\n' 'flamegraph stack filter retained an unresolved frame' >&2
  exit 1
fi
grep -Fq 'unresolved_stack_lines=1' "$work_dir/generated-filter.txt"
python3 scripts/validate-flamegraph.py \
  "$work_dir/valid.svg" "$work_dir/profile.log" "$work_dir/filter.txt" \
  "$work_dir/summary.txt"
grep -Fq 'unresolved_frames=0' "$work_dir/summary.txt"
if python3 scripts/validate-flamegraph.py \
  "$work_dir/invalid.svg" "$work_dir/profile.log" "$work_dir/filter.txt" \
  "$work_dir/invalid-summary.txt" \
  >"$work_dir/invalid.out" 2>&1; then
  printf '%s\n' 'flamegraph validator accepted an unresolved frame' >&2
  exit 1
fi
grep -Fq 'unresolved/null frames found' "$work_dir/invalid.out"
printf '%s\n' 'flamegraph workload and validator tests passed'
