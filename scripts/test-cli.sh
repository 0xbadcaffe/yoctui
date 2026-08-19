#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cli_config_dir="$(mktemp -d)"
trap 'rm -rf "$cli_config_dir"' EXIT

doctor_output="$(
  XDG_CONFIG_HOME="$cli_config_dir" \
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
  output="$(XDG_CONFIG_HOME="$cli_config_dir" cargo run -q -p yoctui -- "$@" 2>&1)"
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
