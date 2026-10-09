#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

test_binary="${YOCTUI_TEST_BINARY:-}"
if [[ -z "$test_binary" ]]; then
  # Keep every terminal layer explicit so a renamed or missing test cannot be
  # hidden by a broad workspace invocation.
  cargo test -q -p yoctui --bin yoctui ux_terminal_runtime
  cargo test -q -p yoctui --test daemon_pty_runtime ux_terminal_real_pty
  cargo test -q -p yoctui --test daemon_state_runtime reattach
  cargo test -q -p yoctui --bin yoctui pty_attach
  cargo test -q -p yoctui --bin yoctui daemon_raw::tests::raw_pty
  cargo test -q -p yoctui-bitbake --lib pty_runner
  cargo test -q -p yoctui-model --lib ux_terminal
  cargo test -q -p yoctui-model --lib pty_session
  cargo test -q -p yoctui-model --lib terminal_emulation
  cargo test -q -p yoctui-app --lib ux_terminal
  cargo test -q -p yoctui-app --lib mouse_runtime_routes_dialog_and_terminal
  cargo test -q -p yoctui-ui --lib ux_terminal
  cargo test -q -p yoctui-protocol --lib next_generation_pty
  cargo test -q -p yoctui-e2e --lib next_generation_pty
  cargo build -q -p yoctui
  test_binary="$repo_root/target/debug/yoctui"
else
  test_binary="$(realpath -- "$test_binary")"
  test -x "$test_binary"
fi

harness_root="$(mktemp -d /tmp/yoctui-workbench-terminal.XXXXXX)"
export XDG_CONFIG_HOME="$harness_root/config"
export XDG_STATE_HOME="$harness_root/state"
export XDG_RUNTIME_DIR="$harness_root/runtime"
export YOCTUI_TERMINAL_GRAPHICS=none
mkdir -m 700 "$XDG_CONFIG_HOME" "$XDG_STATE_HOME" "$XDG_RUNTIME_DIR"
mkdir -m 700 "$harness_root/build"

cleanup() {
  "$test_binary" daemon stop >/dev/null 2>&1 || true
  rm -rf -- "$harness_root"
}
trap cleanup EXIT

"$test_binary" daemon start >/dev/null

python3 "$repo_root/scripts/workbench_terminal_acceptance.py" \
  "$repo_root" "$test_binary" "${YOCTUI_TERMINAL_EVIDENCE:-}"
