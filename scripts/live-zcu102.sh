#!/usr/bin/env bash
# Use only the dedicated AMD validation container/environment, never the user's daemon.
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
validation_root="${YOCTUI_ZCU102_ROOT:-/home/bspguy-dev/src/yoctui-zcu102-2026.1}"
validation_container="${YOCTUI_ZCU102_CONTAINER:-yoctui-zcu102-validation-2026-1}"
binary="$repo_root/target/release/yoctui"

if [[ ! -f "$validation_root/setupsdk" || ! -f "$validation_root/build/conf/bblayers.conf" || ! -x "$binary" ]]; then
  printf 'ZCU102 validation requires the initialized vendor checkout and release binary.\n' >&2
  exit 2
fi
if [[ "$(docker inspect --format '{{.State.Running}}' "$validation_container")" != true ]]; then
  printf 'The dedicated ZCU102 validation container is not running.\n' >&2
  exit 2
fi
case "${1:-status}" in
  start) command_args=(daemon start) ;;
  status) command_args=(daemon status) ;;
  build) command_args=(daemon build petalinux-image-minimal) ;;
  sessions) command_args=(sessions) ;;
  doctor) command_args=(doctor) ;;
  attach) command_args=(attach) ;;
  *) printf 'Usage: %s [start|status|build|sessions|doctor|attach]\n' "$0" >&2; exit 2 ;;
esac
if (( $# > 1 )); then
  printf 'Unexpected extra arguments; this helper is scoped to the isolated validation.\n' >&2
  exit 2
fi

tty_args=()
if [[ "${1:-status}" == attach ]]; then
  if [[ ! -t 0 || ! -t 1 ]]; then
    printf 'Attach requires an interactive terminal.\n' >&2
    exit 2
  fi
  tty_args=(-it)
fi
exec docker exec "${tty_args[@]}" --user 1000:1000 \
  --workdir "$validation_root" \
  -e "XDG_RUNTIME_DIR=$validation_root/runtime" \
  -e "XDG_STATE_HOME=$validation_root/state" \
  -e "YOCTUI_DAEMON_LOG=$validation_root/logs/daemon.log" \
  "$validation_container" bash -c \
  'set -e; source "$1" "$2" >/dev/null; shift 2; exec "$@"' \
  bash "$validation_root/setupsdk" "$validation_root/build" "$binary" "${command_args[@]}"
