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
m22_evidence="$(realpath -m -- "${YOCTUI_M22_EVIDENCE:-$release_evidence_root/m22-concept-live}")"
case "$evidence" in
  "$release_evidence_root"/*) ;;
  *)
    printf 'next-generation UI evidence must stay below artifacts/release-quality: %s\n' "$evidence" >&2
    exit 2
    ;;
esac
case "$m22_evidence" in
  "$release_evidence_root"/*) ;;
  *)
    printf 'M22 concept evidence must stay below artifacts/release-quality: %s\n' "$m22_evidence" >&2
    exit 2
    ;;
esac
test -x "$source_poky/oe-init-build-env"
if [[ "${YOCTUI_LIVE_CONTAINER:-0}" != "1" ]] \
  && command -v unshare >/dev/null 2>&1 \
  && ! unshare -Ur true >/dev/null 2>&1; then
  printf '%s\n' 'next-generation UI live test requires unprivileged user namespaces for BitBake' >&2
  exit 2
fi

work_root="$(mktemp -d "$repo_root/.yoctui-next-ui-live.XXXXXX")"
runtime="$work_root/runtime"
state="$work_root/state"
config="$work_root/config"
build_dir="$work_root/build"
mkdir -m 700 "$runtime" "$state" "$config"
cargo_target_dir="$(realpath -m -- "${CARGO_TARGET_DIR:-$repo_root/target}")"
prebuilt_binary="${YOCTUI_LIVE_PREBUILT_BINARY:-}"
active_capture_pid=""
if [[ -n "$prebuilt_binary" ]]; then
  binary="$(realpath -- "$prebuilt_binary")"
  test -x "$binary"
else
  binary="$cargo_target_dir/release/yoctui"
fi
capture_failure_logs() {
  local output="$evidence/live-build-failure.log"
  local captured=0
  local failed_run=""
  local log

  [[ -d "$build_dir/tmp/work" ]] || return 0
  : >"$output"
  if [[ -s "$evidence/build-status.log" ]]; then
    failed_run="$(sed -n "s/.*Execution of '\([^']*\)'.*/\1/p" \
      "$evidence/build-status.log" | tail -1)"
  fi
  if [[ -n "$failed_run" ]]; then
    for log in "${failed_run/\/run./\/log.}" "$failed_run"; do
      [[ -f "$log" ]] || continue
      printf '\n===== %s =====\n' "${log#"$build_dir"/}" >>"$output"
      tail -c 400000 -- "$log" >>"$output" 2>&1 || true
      captured=$((captured + 1))
    done
  fi
  while IFS= read -r -d '' log; do
    [[ "$log" == "$failed_run" || "$log" == "${failed_run/\/run./\/log.}" ]] && continue
    printf '\n===== %s =====\n' "${log#"$build_dir"/}" >>"$output"
    tail -c 200000 -- "$log" >>"$output" 2>&1 || true
    captured=$((captured + 1))
    (( captured >= 8 )) && break
  done < <(find "$build_dir/tmp/work" -type f \
    \( -name 'log.do_*' -o -name 'run.do_*' \) -mmin -30 -print0 | sort -z)
  if (( captured == 0 )); then
    rm -f -- "$output"
  fi
}
cleanup() {
  local exit_status="$1"
  local daemon_pid=""
  if (( exit_status != 0 )); then
    capture_failure_logs
  fi
  if [[ "$active_capture_pid" =~ ^[1-9][0-9]*$ ]] && kill -0 "$active_capture_pid" 2>/dev/null; then
    kill -TERM "$active_capture_pid" 2>/dev/null || true
    wait "$active_capture_pid" 2>/dev/null || true
  fi
  if [[ -x "$binary" ]]; then
    daemon_pid="$(YOCTUI_BUILD_DIR="$build_dir" XDG_RUNTIME_DIR="$runtime" \
      XDG_STATE_HOME="$state" "$binary" daemon status 2>/dev/null \
      | sed -n 's/^pid: //p' | head -1 || true)"
    YOCTUI_BUILD_DIR="$build_dir" XDG_RUNTIME_DIR="$runtime" \
      XDG_STATE_HOME="$state" "$binary" daemon stop >/dev/null 2>&1 || true
    if [[ "$daemon_pid" =~ ^[1-9][0-9]*$ ]] && kill -0 "$daemon_pid" 2>/dev/null; then
      kill -TERM "$daemon_pid" 2>/dev/null || true
      for _ in {1..50}; do
        kill -0 "$daemon_pid" 2>/dev/null || break
        sleep 0.1
      done
    fi
  fi
  rm -rf -- "$work_root" || {
    sleep 1
    rm -rf -- "$work_root"
  }
}
trap 'exit_status=$?; trap - EXIT; cleanup "$exit_status"; exit "$exit_status"' EXIT
rm -rf "$evidence"
mkdir -p "$evidence"
rm -rf "$m22_evidence"
mkdir -p "$m22_evidence"

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
printf '\nBB_DISKMON_DIRS = ""\nINHERIT += "rm_work"\nBB_NUMBER_THREADS = "4"\nPARALLEL_MAKE = "-j 4"\n' >>"$build_dir/conf/local.conf"
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
if [[ -z "$prebuilt_binary" ]]; then
  CARGO_TARGET_DIR="$cargo_target_dir" cargo build --release -p yoctui >/dev/null
fi
binary_sha256="$(sha256sum "$binary" | cut -d' ' -f1)"
source_commit="$(git rev-parse HEAD)"
bitbake_version="$(bitbake --version | head -1 | tr -d '\r')"
host="$(uname -srm)"
host_distribution="$(. /etc/os-release; printf '%s' "${PRETTY_NAME:-${ID:-unknown}}")"
host_libc="$(getconf GNU_LIBC_VERSION)"

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

python3 "$repo_root/scripts/capture-live-next-generation-ui.py" \
  --binary "$binary" --build-dir "$build_dir" --output "$m22_evidence/idle-dashboard" \
  --mode dashboard --backend process

active_ready="$work_root/active-capture-ready"
python3 "$repo_root/scripts/capture-live-next-generation-ui.py" \
  --binary "$binary" --build-dir "$build_dir" --output "$evidence/active-tasks" \
  --mode tasks --backend process --seconds 600 \
  --expect '▶ Running' --expect 'Log Viewer' --ready-file "$active_ready" &
active_capture_pid="$!"
active_ready_deadline=$((SECONDS + 60))
while [[ ! -s "$active_ready" ]] && (( SECONDS < active_ready_deadline )); do
  kill -0 "$active_capture_pid" 2>/dev/null || wait "$active_capture_pid"
  sleep 1
done
test -s "$active_ready"

: >"$evidence/build-status.log"
"$binary" daemon build core-image-minimal
"$binary" daemon status >>"$evidence/build-status.log" 2>&1 || true
deadline=$((SECONDS + timeout_seconds))
wait "$active_capture_pid"
active_capture_pid=""
grep -Fq '▶ Running' "$evidence/active-tasks.txt"
grep -Fq 'Log Viewer' "$evidence/active-tasks.txt"

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

image_manifest="$(find "$build_dir/tmp/deploy/images/$machine" -maxdepth 1 -type f \
  -name "core-image-minimal-$machine*.manifest" -print | sort | head -1)"
test -n "$image_manifest"
manifest_sha256="$(sha256sum "$image_manifest" | cut -d' ' -f1)"
manifest_bytes="$(wc -c <"$image_manifest")"
manifest_packages="$(wc -l <"$image_manifest")"
sed -n '1,200p' "$image_manifest" >"$evidence/image-manifest-sample.txt"
pkgdata_files="$(find "$build_dir/tmp/pkgdata" -type f 2>/dev/null | wc -l)"
rootfs_state="unavailable_cleaned"
if find "$build_dir/tmp/work" -type d -path '*/core-image-minimal/*/rootfs' -print -quit \
    | grep -q .; then
  rootfs_state="available"
fi
python3 - "$evidence/rootfs-evidence.json" <<PY
import json
from pathlib import Path
Path("$evidence/rootfs-evidence.json").write_text(json.dumps({
  "schema": 1,
  "image": "core-image-minimal",
  "machine": "$machine",
  "manifest_sha256": "$manifest_sha256",
  "manifest_bytes": int("$manifest_bytes"),
  "manifest_packages": int("$manifest_packages"),
  "pkgdata_files": int("$pkgdata_files"),
  "filesystem_rootfs_state": "$rootfs_state",
  "filesystem_rootfs_reason": "rm_work removes transient work roots after the successful image build",
}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

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

cp "$evidence/active-tasks.ansi" "$m22_evidence/active-build-tasks.ansi"
cp "$evidence/active-tasks.txt" "$m22_evidence/active-build-tasks.txt"
cp "$evidence/active-tasks.meta" "$m22_evidence/active-build-tasks.meta"
python3 "$repo_root/scripts/capture-live-m22-concepts.py" \
  --binary "$binary" --build-dir "$build_dir" --output "$m22_evidence/failed-build-errors" \
  --scenario errors --backend process
python3 "$repo_root/scripts/capture-live-m22-concepts.py" \
  --binary "$binary" --build-dir "$build_dir" --output "$m22_evidence/rootfs-composition" \
  --scenario rootfs --backend process
python3 "$repo_root/scripts/capture-live-m22-concepts.py" \
  --binary "$binary" --build-dir "$build_dir" --output "$m22_evidence/editor-application-menu" \
  --scenario editor-menu --backend process
YOCTUI_TEST_BINARY="$binary" YOCTUI_TERMINAL_EVIDENCE="$m22_evidence" \
  "$repo_root/scripts/test-workbench-terminal.sh"

python3 - "$m22_evidence" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
reports = {
    "idle-dashboard": {
        "interactions": ["launch a real client against the idle supported-host daemon"],
        "observed_assertions": ["Current Build · Idle", "Daemon: ✓ Connected", "F10 Menu"],
    },
    "active-build-tasks": {
        "interactions": ["submit core-image-minimal", "press F2 while real BitBake tasks are running"],
        "observed_assertions": ["Tasks: core-image-minimal", "▶ Running", "Log Viewer", "Daemon: ✓ Connected"],
    },
}
for scenario, contract in reports.items():
    contract.update({
        "schema": 1,
        "scenario": scenario,
        "terminal": f"{scenario}.ansi",
        "semantic": f"{scenario}.txt",
    })
    (root / f"{scenario}.report.json").write_text(
        json.dumps(contract, indent=2, sort_keys=True) + "\n", encoding="utf-8"
    )
PY

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
  "host_distribution": "$host_distribution",
  "host_libc": "$host_libc",
  "machine": "$machine",
  "distro": "$distro",
  "yocto_release": "$yocto_release",
  "build_directory": "$build_dir",
  "target": "core-image-minimal",
  "started_utc": "$started_utc",
  "finished_utc": "$finished_utc",
  "scenarios": {name: "passed" for name in (
    "startup", "environment", "recipes", "layers", "menus_and_availability",
    "tasks", "live_logs", "build_completion", "image_manifest_pkgdata_rootfs",
    "safe_failure", "context_terminal", "interactive_task_availability",
    "daemon_reconnect")},
}, indent=2, sort_keys=True) + "\n", encoding="utf-8")
PY

while IFS= read -r -d '' evidence_text; do
  sed -i 's/[[:space:]]\+$//' "$evidence_text"
done < <(find "$evidence" -maxdepth 1 -type f \
  \( -name '*.json' -o -name '*.log' -o -name '*.meta' -o -name '*.txt' \) \
  -print0)
(cd "$evidence" && find . -maxdepth 1 -type f ! -name checksums.sha256 -printf '%P\n' | sort | xargs sha256sum >checksums.sha256)
YOCTUI_LIVE_BINARY="$binary" "$repo_root/scripts/verify-next-generation-ui-evidence.sh"
printf 'next-generation UI live Poky validation passed (%s, %s)\n' "$poky_revision" "$bitbake_version"
