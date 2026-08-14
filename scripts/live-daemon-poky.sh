#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
source_poky="${YOCTUI_POKY_SOURCE:-/home/bspguy-dev/src/poky}"
target="${YOCTUI_LIVE_POKY_TARGET:-core-image-minimal}"
timeout_seconds="${YOCTUI_LIVE_BUILD_TIMEOUT:-900}"
if [[ ! -x "$source_poky/oe-init-build-env" ]]; then
  printf 'live daemon Poky: source is unavailable: %s\n' "$source_poky" >&2
  exit 2
fi
if command -v unshare >/dev/null 2>&1 && ! unshare -Ur true >/dev/null 2>&1; then
  printf '%s\n' \
    'live daemon Poky: host rejects unprivileged user namespaces required by Poky BitBake (often AppArmor); run on a supported host or adjust the host policy.' >&2
  exit 2
fi

work_root="$(mktemp -d "$repo_root/.yoctui-live-daemon.XXXXXX")"
runtime="$work_root/runtime"
state="$work_root/state"
build_dir="$work_root/build"
daemon_log="$work_root/daemon.log"
mkdir -p "$runtime" "$state"
chmod 700 "$runtime" "$state"
trap 'YOCTUI_BUILD_DIR="$build_dir" XDG_RUNTIME_DIR="$runtime" XDG_STATE_HOME="$state" "$repo_root/target/debug/yoctui" daemon stop >/dev/null 2>&1 || true; rm -rf "$work_root"' EXIT

git clone --local --no-hardlinks "$source_poky" "$work_root/poky" >/dev/null
set +u
source "$work_root/poky/oe-init-build-env" "$build_dir" >/dev/null
set -u
if ! command -v lz4c >/dev/null 2>&1; then
  printf '\nHOSTTOOLS:remove = "lz4c"\n' >>"$build_dir/conf/local.conf"
fi
if [[ -n "${YOCTUI_LIVE_CACHE:-}" ]]; then
  cache_root="${YOCTUI_LIVE_CACHE}"
  mkdir -p "$cache_root/downloads" "$cache_root/sstate-cache"
  printf '\nDL_DIR = "%s/downloads"\nSSTATE_DIR = "%s/sstate-cache"\n' \
    "$cache_root" "$cache_root" >>"$build_dir/conf/local.conf"
fi
export YOCTUI_BUILD_DIR="$build_dir"
export XDG_RUNTIME_DIR="$runtime"
export XDG_STATE_HOME="$state"
export YOCTUI_DAEMON_LOG="$daemon_log"

print_build_diagnostics() {
  printf 'live daemon Poky: BitBake diagnostics:\n' >&2
  if [[ -d "$build_dir/tmp/log/cooker" ]]; then
    find "$build_dir/tmp/log/cooker" -type f -maxdepth 2 -print0 2>/dev/null |
      xargs -0r grep -HnE '(^|[[:space:]])ERROR:|Task .* failed|FetchError' 2>/dev/null |
      tail -80 >&2 || true
  fi
  tail -80 "$build_dir/bitbake-cookerdaemon.log" >&2 2>/dev/null || true
}

binary="${YOCTUI_LIVE_BINARY:-$repo_root/target/debug/yoctui}"
if [[ ! -x "$binary" ]]; then
  cargo build -p yoctui >/dev/null
  binary="$repo_root/target/debug/yoctui"
fi
"$binary" daemon start
"$binary" daemon build "$target"

deadline=$((SECONDS + timeout_seconds))
seen_running=0
while (( SECONDS < deadline )); do
  status="$("$binary" daemon status 2>&1 || true)"
  printf '%s\n' "$status"
  if grep -q 'daemon unavailable\|daemon is not running' <<<"$status"; then
    printf 'live daemon Poky: daemon disappeared; diagnostic:\n' >&2
    tail -80 "$daemon_log" >&2 || true
    exit 1
  fi
  grep -q 'job .*Running\|lifecycle: Running' <<<"$status" && seen_running=1 || true
  if grep -q 'job .*Exited\|lifecycle: Exited' <<<"$status"; then
    (( seen_running == 1 )) || { printf 'live daemon Poky: build never reported Running\n' >&2; exit 1; }
    printf 'live daemon Poky: detached daemon build completed\n'
    exit 0
  fi
  if grep -q 'job .*Failed\|lifecycle: Failed' <<<"$status"; then
    printf 'live daemon Poky: daemon build failed\n' >&2
    print_build_diagnostics
    exit 1
  fi
  # Keep status attach traffic bounded while BitBake is emitting a large
  # stream of task events; a reconnect probe every ten seconds is sufficient
  # to prove detach/reconnect without competing with the build worker.
  sleep 10
done
printf 'live daemon Poky: timed out after %ss\n' "$timeout_seconds" >&2
exit 1
