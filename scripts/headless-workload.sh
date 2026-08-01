#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
binary="${1:-target/debug/yoctui}"
backend="${2:-bridge}"
workload_config_dir="$(mktemp -d)"
trap 'rm -rf "$workload_config_dir"' EXIT
XDG_CONFIG_HOME="$workload_config_dir" \
"$binary" --headless --backend "$backend" --build-dir "$repo_root"
