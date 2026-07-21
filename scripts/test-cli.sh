#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
output="$(cargo run -q -p yoctui -- --backend bridge --build-dir "$repo_root" --headless)"
if [[ "$output" != *"headless inspection completed"* ]]; then
  printf '%s\n' 'headless bridge inspection did not complete' >&2
  exit 1
fi

inspect="$(cargo run -q -p yoctui -- --backend bridge --build-dir "$repo_root" inspect)"
if [[ "$inspect" != *"build directory:"* ]]; then
  printf '%s\n' 'bridge inspection did not report a build directory' >&2
  exit 1
fi

config_output="$(YOCTUI_VARIABLE_PROVENANCE_JSON='{"PATH":"conf/local.conf:8"}' cargo run -q -p yoctui -- --backend bridge --build-dir "$repo_root" config PATH)"
if [[ "$config_output" != *"PATH="* || "$config_output" != *"provenance: conf/local.conf:8"* ]]; then
  printf '%s\n' 'bridge variable query did not report its value and provenance' >&2
  exit 1
fi

fixture_dir="$(mktemp -d)"
trap 'rm -rf "$fixture_dir"' EXIT
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
build_output="$(PYTHONPATH="$fixture_dir" cargo run -q -p yoctui -- --backend bridge --build-dir "$repo_root" --headless core-image-minimal)"
if [[ "$build_output" != *"build completed"* ]]; then
  printf '%s\n' 'fake bridge build did not complete' >&2
  exit 1
fi
