#!/usr/bin/env bash
set -euo pipefail

cargo build -q -p yoctui
capture="$(mktemp)"
terminal_config_dir="$(mktemp -d)"
trap 'rm -f "$capture"; rm -rf "$terminal_config_dir"' EXIT

printf q | XDG_CONFIG_HOME="$terminal_config_dir" \
  script -qec 'target/debug/yoctui --backend bridge' /dev/null >"$capture"
output="$(<"$capture")"

for sequence in $'\e[?1049h' $'\e[?1049l' $'\e[?25l' $'\e[?25h'; do
  if [[ "$output" != *"$sequence"* ]]; then
    printf '%s\n' 'terminal lifecycle sequence was not observed' >&2
    exit 1
  fi
done
