#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

if [[ -n "${YOCTUI_WORKBENCH_LIVE_SOURCE:-}" ]]; then
  source_poky="$(realpath -- "$YOCTUI_WORKBENCH_LIVE_SOURCE")"
  test -x "$source_poky/oe-init-build-env"
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/target/ui-performance}"
  YOCTUI_POKY_SOURCE="$source_poky" ./scripts/test-live-next-generation-ui.sh
fi

cargo test -q -p yoctui-model --lib ux_rootfs
cargo test -q -p yoctui-model --lib ux_terminal
cargo test -q -p yoctui-app --lib ux_menu
cargo test -q -p yoctui-ui --lib ux_rootfs
cargo test -q -p yoctui-ui --lib ux_terminal

./scripts/verify-live-workbench-ux-evidence.sh
printf '%s\n' 'live one-stop workbench UX gate passed'
