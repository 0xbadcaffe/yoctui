#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"

if [[ "${YOCTUI_LIVE_BITBAKE:-0}" != 1 ]]; then
  printf '%s\n' \
    'SKIP live BitBake smoke: set YOCTUI_LIVE_BITBAKE=1 and YOCTUI_LIVE_BUILD_DIR=/path/to/build to enable it.'
  exit 0
fi

build_dir="${YOCTUI_LIVE_BUILD_DIR:-${BUILDDIR:-}}"
if [[ -z "$build_dir" ]]; then
  printf '%s\n' \
    'live BitBake smoke: YOCTUI_LIVE_BUILD_DIR is required when YOCTUI_LIVE_BITBAKE=1' >&2
  exit 2
fi
if [[ ! -d "$build_dir" ]]; then
  printf 'live BitBake smoke: build directory does not exist: %s\n' "$build_dir" >&2
  exit 2
fi
build_dir="$(cd "$build_dir" && pwd -P)"
if [[ ! -f "$build_dir/conf/bblayers.conf" || ! -f "$build_dir/conf/local.conf" ]]; then
  printf 'live BitBake smoke: not an initialized build directory (missing conf/bblayers.conf or conf/local.conf): %s\n' "$build_dir" >&2
  exit 2
fi

if [[ -f "$build_dir/init-build-env" ]]; then
  # bitbake-setup workspaces provide a reproducible environment wrapper.
  set +u
  source "$build_dir/init-build-env" >/dev/null
  set -u
elif [[ -n "${YOCTUI_OE_INIT_BUILD_ENV:-}" && -f "$YOCTUI_OE_INIT_BUILD_ENV" ]]; then
  set +u
  source "$YOCTUI_OE_INIT_BUILD_ENV" "$build_dir" >/dev/null
  set -u
elif [[ "${BUILDDIR:-}" != "$build_dir" ]]; then
  printf '%s\n' \
    'live BitBake smoke: no init-build-env wrapper found; source oe-init-build-env first or set YOCTUI_OE_INIT_BUILD_ENV' >&2
  exit 2
fi

if ! command -v bitbake >/dev/null 2>&1; then
  printf '%s\n' 'live BitBake smoke: bitbake is unavailable after environment initialization' >&2
  exit 2
fi
if ! python3 -c 'import bb, bb.tinfoil' >/dev/null 2>&1; then
  printf '%s\n' 'live BitBake smoke: Python cannot import bb.tinfoil after environment initialization' >&2
  exit 2
fi

target="${YOCTUI_LIVE_TARGET:-base-files}"
task="${YOCTUI_LIVE_TASK:-listtasks}"
cancel_target="${YOCTUI_LIVE_CANCEL_TARGET:-core-image-minimal}"
timeout="${YOCTUI_LIVE_TIMEOUT:-300}"

printf 'live BitBake smoke: build=%s target=%s task=%s cancel-target=%s\n' \
  "$build_dir" "$target" "$task" "$cancel_target"

exec python3 "$repo_root/scripts/live_bitbake_smoke.py" \
  --bridge "$repo_root/bridge/yoctui_bridge.py" \
  --build-dir "$build_dir" \
  --target "$target" \
  --task "$task" \
  --cancel-target "$cancel_target" \
  --timeout "$timeout"
