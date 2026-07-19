#!/usr/bin/env bash
set -euo pipefail

cargo build -q -p yoctui
capture="$(mktemp)"
trap 'rm -f "$capture"' EXIT

printf q | script -qec 'target/debug/yoctui --backend bridge' /dev/null >"$capture"
output="$(<"$capture")"

for sequence in $'\e[?1049h' $'\e[?1049l' $'\e[?25l' $'\e[?25h'; do
  if [[ "$output" != *"$sequence"* ]]; then
    printf '%s\n' 'terminal lifecycle sequence was not observed' >&2
    exit 1
  fi
done
