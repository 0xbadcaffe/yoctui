#!/usr/bin/env bash
# Pure checks for the committed v0.1.309 CLI's packaged public help contract.

yoctui_verify_packaged_help() {
  local package_help="${1-}"
  local marker
  [[ "${package_help%%$'\n'*}" == 'A Terminal Workbanch for Yocto/BitBake' ]] || return 1
  for marker in \
    'Usage: yoctui [OPTIONS] [TARGETS]... [COMMAND]' \
    $'\nCommands:\n' \
    $'\n  doctor ' \
    $'\n  attach ' \
    $'\n  daemon ' \
    $'\nOptions:\n' \
    '--build-dir <BUILD_DIR>' \
    '--config <CONFIG>' \
    '--version'; do
    [[ "$package_help" == *"$marker"* ]] || return 1
  done
}
