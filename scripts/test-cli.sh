#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cli_config_dir="$(mktemp -d)"
trap 'rm -rf "$cli_config_dir"' EXIT

output="$(XDG_CONFIG_HOME="$cli_config_dir" cargo run -q -p yoctui -- --backend bridge --build-dir "$repo_root" --headless)"
if [[ "$output" != *"headless inspection completed"* ]]; then
  printf '%s\n' 'headless bridge inspection did not complete' >&2
  exit 1
fi

inspect="$(XDG_CONFIG_HOME="$cli_config_dir" cargo run -q -p yoctui -- --backend bridge --build-dir "$repo_root" inspect)"
if [[ "$inspect" != *"build directory:"* ]]; then
  printf '%s\n' 'bridge inspection did not report a build directory' >&2
  exit 1
fi

config_output="$(XDG_CONFIG_HOME="$cli_config_dir" YOCTUI_VARIABLE_PROVENANCE_JSON='{"PATH":"conf/local.conf:8"}' cargo run -q -p yoctui -- --backend bridge --build-dir "$repo_root" config PATH)"
if [[ "$config_output" != *"PATH="* || "$config_output" != *"provenance: conf/local.conf:8"* ]]; then
  printf '%s\n' 'bridge variable query did not report its value and provenance' >&2
  exit 1
fi

fixture_dir="$cli_config_dir/fixture"
mkdir -p "$fixture_dir"
printf '%s\n' \
  '__version__ = "2.8.1"' \
  'class BuildCompleted:' \
  ' def __init__(self): self.success = True' \
  'class Connection:' \
  ' def start_build(self, targets, task): pass' \
  ' def drain_events(self): return [BuildCompleted()]' \
  'class Server:' \
  ' def connect(self): return Connection()' \
  'server = Server()' > "$fixture_dir/bb.py"
build_output="$(XDG_CONFIG_HOME="$cli_config_dir" PYTHONPATH="$fixture_dir" cargo run -q -p yoctui -- --backend bridge --build-dir "$repo_root" --headless core-image-minimal)"
if [[ "$build_output" != *"build completed"* ]]; then
  printf '%s\n' 'fake bridge build did not complete' >&2
  exit 1
fi
