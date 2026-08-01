#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

stress_iterations="${YOCTUI_STRESS_ITERATIONS:-3}"
case "$stress_iterations" in
  ''|*[!0-9]*)
    printf '%s\n' 'YOCTUI_STRESS_ITERATIONS must be an integer from 1 through 20' >&2
    exit 2
    ;;
esac
if ((stress_iterations < 1 || stress_iterations > 20)); then
  printf '%s\n' 'YOCTUI_STRESS_ITERATIONS must be an integer from 1 through 20' >&2
  exit 2
fi

for ((iteration = 1; iteration <= stress_iterations; iteration++)); do
  printf 'stress iteration %d/%d\n' "$iteration" "$stress_iterations"
  cargo test -p yoctui-model hardening_stress_model_retention
  cargo test -p yoctui-protocol hardening_stress_protocol
  cargo test -p yoctui-bitbake hardening_stress_process_tree
done
