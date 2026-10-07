#!/usr/bin/env bash
# Parse only, using the daemon's exact menuconfig environment/cache identity.
# No image/task build occurs here; builds must use live-zcu102.sh build.
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
validation_root="${YOCTUI_ZCU102_ROOT:-$HOME/src/yoctui-zcu102-2026.1}"
validation_container="${YOCTUI_ZCU102_CONTAINER:-yoctui-zcu102-validation-2026-1}"
binary="$repo_root/target/release/yoctui"
if (( $# != 0 )) || [[ ! -f "$validation_root/build/conf/bblayers.conf" || ! -x "$binary" ]]; then
  printf 'Requires the initialized isolated validation build and release; no arguments accepted.\n' >&2
  exit 2
fi
# The daemon canonicalizes current_exe inside the container. A symlinked Cargo
# target directory must produce the same OE_TERMINAL_CUSTOMCMD/cache identity.
binary="$(docker exec --user 1000:1000 "$validation_container" readlink -e -- "$binary")"
if [[ "$binary" != /* ]]; then
  printf 'Cannot resolve the release binary inside the validation container.\n' >&2
  exit 2
fi
# These paths are serialized into OE_TERMINAL_CUSTOMCMD, not evaluated here.
if [[ "$binary$validation_root" == *"'"* || "$binary$validation_root" == *$'\n'* ]]; then
  printf 'Validation paths containing quotes/newlines are unsupported by this helper.\n' >&2
  exit 2
fi
handoff="'$binary' __menuconfig-handoff --socket '$validation_root/runtime/yoctui/menuconfig.sock' -- {command}"
exec docker exec --user 1000:1000 --workdir "$validation_root" \
  -e "OE_TERMINAL=custom" -e "OE_TERMINAL_CUSTOMCMD=$handoff" \
  "$validation_container" bash -c \
  'set -e; source "$1" "$2" >/dev/null; export BB_ENV_PASSTHROUGH_ADDITIONS="$BB_ENV_PASSTHROUGH_ADDITIONS OE_TERMINAL OE_TERMINAL_CUSTOMCMD"; exec script -q -e -f -c "bitbake -p" "$3"' \
  bash "$validation_root/setupsdk" "$validation_root/build" "$validation_root/logs/daemon-compatible-parse-only.log"
