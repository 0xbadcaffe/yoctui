#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
source "$repo_root/scripts/cratesio-package-contract.sh"

fixture='A Terminal Workbanch for Yocto/BitBake

Usage: yoctui [OPTIONS] [TARGETS]... [COMMAND]

Commands:
  doctor    
  attach    Attach the interactive client to the persistent daemon
  daemon    

Options:
      --build-dir <BUILD_DIR>
      --config <CONFIG>
  -V, --version                Print version'

yoctui_verify_packaged_help "$fixture"
reject() {
  if yoctui_verify_packaged_help "$1"; then
    printf 'invalid packaged help unexpectedly accepted: %s\n' "$2" >&2
    exit 1
  fi
}

reject '' 'empty help'
reject 'Ratatui frontend and control client for BitBake' 'obsolete banner only'
reject 'A Terminal Workbanch for Yocto/BitBake' 'current banner without commands'
reject "${fixture/'A Terminal Workbanch for Yocto/BitBake'/'Ratatui frontend and control client for BitBake'}" 'old description with current command list'
reject "prefix$fixture" 'unrelated program prefix'

for missing in \
  'Usage: yoctui [OPTIONS] [TARGETS]... [COMMAND]' \
  $'\nCommands:\n' \
  $'\n  doctor ' \
  $'\n  attach ' \
  $'\n  daemon ' \
  $'\nOptions:\n' \
  '--build-dir <BUILD_DIR>' \
  '--config <CONFIG>' \
  '--version'; do
  reject "${fixture/"$missing"/}" "missing $missing"
done

printf '%s\n' 'packaged help contract: current help accepted; 14 incomplete/obsolete negatives rejected'
