#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
source_poky="${YOCTUI_POKY_SOURCE:-/home/bspguy-dev/src/poky}"
timeout_seconds="${YOCTUI_LIVE_BUILD_TIMEOUT:-14400}"
if [[ ! "$timeout_seconds" =~ ^[1-9][0-9]*$ ]]; then
  printf 'YOCTUI_LIVE_BUILD_TIMEOUT must be a positive integer: %s\n' "$timeout_seconds" >&2
  exit 2
fi
release_evidence_root="$repo_root/artifacts/release-quality"
evidence="$(realpath -m -- "${YOCTUI_NEXT_UI_EVIDENCE:-$release_evidence_root/next-generation-ui}")"
case "$evidence" in
  "$release_evidence_root"/*) ;;
  *)
    printf 'next-generation UI evidence must stay below artifacts/release-quality: %s\n' "$evidence" >&2
    exit 2
    ;;
esac
test -x "$source_poky/oe-init-build-env"
if command -v unshare >/dev/null 2>&1 && ! unshare -Ur true >/dev/null 2>&1; then
  printf '%s\n' 'next-generation UI live test requires unprivileged user namespaces for BitBake' >&2
  exit 2
fi

work_root="$(mktemp -d "$repo_root/.yoctui-next-ui-live.XXXXXX")"
runtime="$work_root/runtime"
state="$work_root/state"
config="$work_root/config"
build_dir="$work_root/build"
mkdir -m 700 "$runtime" "$state" "$config"
trap 'YOCTUI_BUILD_DIR="$build_dir" XDG_RUNTIME_DIR="$runtime" XDG_STATE_HOME="$state" "$repo_root/target/release/yoctui" daemon stop >/dev/null 2>&1 || true; rm -rf "$work_root"' EXIT
rm -rf "$evidence"
mkdir -p "$evidence"

started_utc="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
git clone --local --no-hardlinks "$source_poky" "$work_root/poky" >/dev/null
poky_revision="$(git -C "$work_root/poky" rev-parse HEAD)"
poky_branch="$(git -C "$source_poky" branch --show-current)"
if [[ -z "$poky_branch" ]]; then
  poky_branch="$(git -C "$source_poky" describe --tags --exact-match HEAD)"
fi
set +u
source "$work_root/poky/oe-init-build-env" "$build_dir" >/dev/null
set -u
unset PYENV_DIR PYENV_HOOK_PATH PYENV_VERSION BBPATH
export PATH="/usr/bin:/bin:$PATH"
export YOCTUI_BUILD_DIR="$build_dir"
export XDG_RUNTIME_DIR="$runtime"
export XDG_STATE_HOME="$state"
export XDG_CONFIG_HOME="$config"
export YOCTUI_DAEMON_LOG="$evidence/daemon.log"
printf '\nBB_DISKMON_DIRS = ""\nINHERIT += "rm_work"\n' >>"$build_dir/conf/local.conf"
# The live target never launches QEMU; omit its optional graphical host stack so
# a cold image build does not compile Mesa/LLVM solely for an unused display.
printf 'PACKAGECONFIG:remove:pn-qemu-native = "sdl virglrenderer epoxy"\n' >>"$build_dir/conf/local.conf"
printf 'PACKAGECONFIG:remove:pn-qemu-system-native = "sdl virglrenderer epoxy"\n' >>"$build_dir/conf/local.conf"
if ! command -v lz4c >/dev/null 2>&1; then
  printf 'HOSTTOOLS:remove = "lz4c"\n' >>"$build_dir/conf/local.conf"
fi
cache_source="${YOCTUI_LIVE_CACHE_SOURCE:-$repo_root/.yoctui-live-cache}"
if [[ -d "$cache_source/downloads" && -w "$cache_source/downloads" ]]; then
  printf 'DL_DIR = "%s/downloads"\n' "$cache_source" >>"$build_dir/conf/local.conf"
fi
if [[ -d "$cache_source/sstate-cache" && -w "$cache_source/sstate-cache" ]]; then
  printf 'SSTATE_DIR = "%s/sstate-cache"\n' "$cache_source" >>"$build_dir/conf/local.conf"
fi

cd "$repo_root"
cargo build --release -p yoctui >/dev/null
binary="$repo_root/target/release/yoctui"
binary_sha256="$(sha256sum "$binary" | cut -d' ' -f1)"
source_commit="$(git rev-parse HEAD)"
bitbake_version="$(bitbake --version | head -1 | tr -d '\r')"
host="$(uname -srm)"

"$binary" daemon start | tee -a "$evidence/daemon.log"
"$binary" --backend bridge --build-dir "$build_dir" doctor >"$evidence/doctor.txt" 2>&1
"$binary" --backend bridge --build-dir "$build_dir" inspect >"$evidence/inspect.txt"
sed -i 's/[[:space:]]\+$//' "$evidence/inspect.txt"
"$binary" --backend bridge --build-dir "$build_dir" layers >"$evidence/layers.txt"
"$binary" --backend bridge --build-dir "$build_dir" recipes >"$evidence/recipes.txt"
grep -Fq 'MACHINE=qemux86-64' "$evidence/inspect.txt"
grep -Fq 'DISTRO=poky' "$evidence/inspect.txt"
grep -Eq '^core-image-minimal[[:space:]]' "$evidence/recipes.txt"
grep -Eq '^busybox[[:space:]]' "$evidence/recipes.txt"
machine="$(sed -n 's/^MACHINE=//p' "$evidence/inspect.txt" | head -1)"
distro="$(sed -n 's/^DISTRO=//p' "$evidence/inspect.txt" | head -1)"
yocto_release="$(sed -n 's/^Yocto\/OpenEmbedded release: //p' "$evidence/inspect.txt" | head -1)"

: >"$evidence/build-status.log"
"$binary" daemon build core-image-minimal
"$binary" daemon status >>"$evidence/build-status.log" 2>&1 || true
deadline=$((SECONDS + timeout_seconds))
active_deadline=$((SECONDS + 180))
captured_active_task=0
while (( SECONDS < active_deadline && SECONDS < deadline )); do
  python3 "$repo_root/scripts/capture-live-next-generation-ui.py" \
    --binary "$binary" --build-dir "$build_dir" --output "$evidence/active-tasks" \
    --mode tasks --backend process --seconds 0.5
  if grep -Fq '▶ Running' "$evidence/active-tasks.txt" && \
    grep -Fq 'Log Viewer' "$evidence/active-tasks.txt"; then
    captured_active_task=1
    break
  fi
  status="$($binary daemon status 2>&1 || true)"
  printf '%s\n' "$status" >>"$evidence/build-status.log"
  grep -q 'job .*Failed\|job .*Lost\|job .*Exited' <<<"$status" && break
done
(( captured_active_task == 1 ))

seen_running=0
grep -q 'job .*Running' "$evidence/build-status.log" && seen_running=1 || true
while (( SECONDS < deadline )); do
  status="$($binary daemon status 2>&1 || true)"
  printf '%s\n' "$status" >>"$evidence/build-status.log"
  if (( $(wc -c <"$evidence/build-status.log") > 1500000 )); then
    tail -c 1000000 "$evidence/build-status.log" >"$evidence/build-status.log.tmp"
    mv "$evidence/build-status.log.tmp" "$evidence/build-status.log"
  fi
  grep -q 'job .*Running' <<<"$status" && seen_running=1 || true
  if grep -q 'job .*Exited' <<<"$status"; then
    break
  fi
  if grep -q 'job .*Failed\|job .*Lost' <<<"$status"; then
    printf '%s\n' "$status" >&2
    exit 1
  fi
  sleep 5
done
grep -q 'job .*Exited' "$evidence/build-status.log"
(( seen_running == 1 ))
python3 "$repo_root/scripts/capture-live-next-generation-ui.py" \
  --binary "$binary" --build-dir "$build_dir" --output "$evidence/completion" --mode tasks --backend process

"$binary" daemon build yoctui-intentional-missing-target
failure_deadline=$((SECONDS + 120))
while (( SECONDS < failure_deadline )); do
  "$binary" daemon status >"$evidence/failure-status.txt" 2>&1 || true
  grep -q 'job .*Failed' "$evidence/failure-status.txt" && break
  sleep 2
done
grep -q 'job .*Failed' "$evidence/failure-status.txt"
python3 "$repo_root/scripts/capture-live-next-generation-ui.py" \
  --binary "$binary" --build-dir "$build_dir" --output "$evidence/failed-task" --mode tasks --backend process

python3 "$repo_root/scripts/capture-live-next-generation-ui.py" \
  --binary "$binary" --build-dir "$build_dir" --output "$evidence/terminal" --mode terminal --backend process
"$binary" daemon status >"$evidence/reconnect-status.txt"
python3 "$repo_root/scripts/capture-live-next-generation-ui.py" \
  --binary "$binary" --build-dir "$build_dir" --output "$evidence/reconnect" --mode dashboard --backend process

finished_utc="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
python3 - "$evidence/manifest.json" <<PY
import json
from pathlib import Path
Path("$evidence/manifest.json").write_text(json.dumps({
  "schema": 1,
  "label": "live",
  "source_commit": "$source_commit",
  "binary_sha256": "$binary_sha256",
  "poky_revision": "$poky_revision",
  "poky_branch": "$poky_branch",
  "bitbake_version": "$bitbake_version",
  "host": "$host",
  "machine": "$machine",
  "distro": "$distro",
  "yocto_release": "$yocto_release",
  "build_directory": "$build_dir",
  "target": "core-image-minimal",
  "started_utc": "$started_utc",
  "finished_utc": "$finished_utc",
  "scenarios": {name: "passed" for name in (
    "startup", "environment", "recipes", "layers", "tasks", "live_logs",
    "build_completion", "safe_failure", "terminal", "daemon_reconnect")},
}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

(cd "$evidence" && find . -maxdepth 1 -type f ! -name checksums.sha256 -printf '%P\n' | sort | xargs sha256sum >checksums.sha256)
"$repo_root/scripts/verify-next-generation-ui-evidence.sh"
printf 'next-generation UI live Poky validation passed (%s, %s)\n' "$poky_revision" "$bitbake_version"
