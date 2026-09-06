#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cli_test_root="$(mktemp -d)"
cli_config_dir="$cli_test_root/config"
cli_runtime_dir="$cli_test_root/runtime"
mkdir -m 700 "$cli_config_dir" "$cli_runtime_dir"
trap 'rm -rf "$cli_test_root"' EXIT

doctor_output="$(
  XDG_CONFIG_HOME="$cli_config_dir" \
    XDG_RUNTIME_DIR="$cli_runtime_dir" \
    cargo run -q -p yoctui -- --backend bridge doctor
)"
for expected in 'bridge protocol: ok' 'compatibility report:' 'authority: Unavailable'; do
  if [[ "$doctor_output" != *"$expected"* ]]; then
    printf 'daemon-independent Doctor output is missing: %s\n' "$expected" >&2
    exit 1
  fi
done

assert_daemon_authority_required() {
  local output status
  set +e
  output="$(
    XDG_CONFIG_HOME="$cli_config_dir" \
      XDG_RUNTIME_DIR="$cli_runtime_dir" \
      cargo run -q -p yoctui -- "$@" 2>&1
  )"
  status="$?"
  set -e
  if ((status == 0)); then
    printf 'direct BitBake operation unexpectedly succeeded: %s\n' "$*" >&2
    exit 1
  fi
  if [[ "$output" != *'BitBake operations require the daemon-owned compatibility snapshot'* ]]; then
    printf '%s\n' "$output" >&2
    printf 'direct BitBake operation omitted daemon authority failure: %s\n' "$*" >&2
    exit 1
  fi
}

assert_daemon_authority_required --backend bridge --build-dir "$repo_root" --headless
assert_daemon_authority_required --backend bridge --build-dir "$repo_root" inspect
assert_daemon_authority_required --backend bridge --build-dir "$repo_root" config PATH
assert_daemon_authority_required \
  --backend bridge --build-dir "$repo_root" --headless core-image-minimal

printf '%s\n' 'CLI Doctor and daemon-authority smoke checks passed'
