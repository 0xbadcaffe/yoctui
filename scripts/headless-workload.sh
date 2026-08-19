#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
binary="${1:-target/debug/yoctui}"
backend="${2:-bridge}"
workload_config_dir="$(mktemp -d)"
trap 'rm -rf "$workload_config_dir"' EXIT
if [[ "$backend" == "bridge" ]]; then
  output="$(
    XDG_CONFIG_HOME="$workload_config_dir" \
    "$binary" --backend bridge doctor
  )"
  if [[ "$output" != *"bridge protocol: ok"* ]]; then
    printf '%s\n' "$output" >&2
    printf '%s\n' 'headless bridge diagnostic did not complete its protocol lifecycle' >&2
    exit 1
  fi
  if [[ "$output" != *"compatibility report:"* ]]; then
    printf '%s\n' "$output" >&2
    printf '%s\n' 'headless bridge diagnostic omitted compatibility authority state' >&2
    exit 1
  fi
  printf '%s\n' 'headless diagnostic completed'
else
  XDG_CONFIG_HOME="$workload_config_dir" \
  "$binary" --headless --backend "$backend" --build-dir "$repo_root"
fi
