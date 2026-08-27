#!/usr/bin/env bash
set -euo pipefail

cargo build -q -p yoctui
terminal_test_root="$(mktemp -d)"
capture="$terminal_test_root/capture"
terminal_input="$terminal_test_root/input"
terminal_config_dir="$terminal_test_root/config"
terminal_runtime_dir="$terminal_test_root/runtime"
terminal_state_dir="$terminal_test_root/state"
runner_pid=""
cleanup() {
  if [[ -n "$runner_pid" ]] && kill -0 "$runner_pid" 2>/dev/null; then
    kill "$runner_pid" 2>/dev/null || true
    wait "$runner_pid" 2>/dev/null || true
  fi
  rm -rf "$terminal_test_root"
}
trap cleanup EXIT

mkdir "$terminal_config_dir" "$terminal_runtime_dir" "$terminal_state_dir"
chmod 700 "$terminal_config_dir" "$terminal_runtime_dir" "$terminal_state_dir"
mkfifo "$terminal_input"
exec 3<>"$terminal_input"
XDG_CONFIG_HOME="$terminal_config_dir" \
XDG_RUNTIME_DIR="$terminal_runtime_dir" \
XDG_STATE_HOME="$terminal_state_dir" \
  timeout --kill-after=2s 10s \
  script -qec 'target/debug/yoctui --backend bridge' /dev/null \
  <"$terminal_input" >"$capture" &
runner_pid="$!"

terminal_ready=false
for ((attempt = 0; attempt < 200; attempt++)); do
  if grep -Fq $'\e[?1049h' "$capture"; then
    terminal_ready=true
    break
  fi
  if ! kill -0 "$runner_pid" 2>/dev/null; then
    break
  fi
  sleep 0.05
done
if [[ "$terminal_ready" != true ]]; then
  printf '%s\n' 'terminal did not enter alternate-screen mode before its deadline' >&2
  exit 1
fi

# A new isolated session opens onboarding by design. Dismiss the innermost
# modal first (Esc is harmless when onboarding is already complete), then use
# the documented quit request and confirmation route.
sleep 0.5
printf '\033' >&3
sleep 0.5
printf '\033' >&3
sleep 0.2
printf q >&3
sleep 0.2
if kill -0 "$runner_pid" 2>/dev/null; then
  # An active external build requires the explicit destructive quit key.
  printf Y >&3
fi
set +e
wait "$runner_pid"
runner_status="$?"
set -e
runner_pid=""
exec 3>&-
if ((runner_status != 0)); then
  printf 'terminal lifecycle command failed or timed out (status %s)\n' "$runner_status" >&2
  exit 1
fi
output="$(<"$capture")"

for sequence in $'\e[?1049h' $'\e[?1049l' $'\e[?25l' $'\e[?25h'; do
  if [[ "$output" != *"$sequence"* ]]; then
    printf '%s\n' 'terminal lifecycle sequence was not observed' >&2
    exit 1
  fi
done
