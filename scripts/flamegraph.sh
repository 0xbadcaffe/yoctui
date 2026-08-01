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
flamegraph_config_dir="$(mktemp -d)"
trap 'rm -rf "$flamegraph_config_dir"' EXIT
XDG_CONFIG_HOME="$flamegraph_config_dir" \
cargo flamegraph --deterministic --output artifacts/flamegraph/yoctui.svg --bin yoctui -- --headless --backend process --build-dir "$repo_root"
