#!/usr/bin/env bash
set -euo pipefail
repo_root="$(git rev-parse --show-toplevel)"
source_poky="${YOCTUI_POKY_SOURCE:-/home/bspguy-dev/src/poky}"
test -x "$source_poky/oe-init-build-env"
root="$(mktemp -d "$repo_root/.yoctui-fresh-poky.XXXXXX")"
trap 'rm -rf "$root"' EXIT
git clone --local --no-hardlinks "$source_poky" "$root/poky" >/dev/null
build_dir="$root/build"
mkdir -p "$build_dir"
# Source the same environment used by a real operator, then run the clean
# checkout's bounded doctor and headless inspection against the isolated build.
set +u
source "$root/poky/oe-init-build-env" "$build_dir" >/dev/null
set -u
unset BBPATH
mkdir -p "$repo_root/artifacts/release-quality/poky"
{ printf 'poky_revision='; git -C "$root/poky" rev-parse HEAD; printf 'poky_branch='; git -C "$root/poky" branch --show-current; printf 'build_dir=%s\n' "$build_dir"; timeout 30s "$repo_root/target/debug/yoctui" --backend process --build-dir "$build_dir" doctor; } >"$repo_root/artifacts/release-quality/poky/doctor.txt" 2>&1
if grep -Eq 'PermissionError|Unable to connect to bitbake server|bridge protocol: failed' "$repo_root/artifacts/release-quality/poky/doctor.txt"; then
  echo "fresh Poky workflow blocked: BitBake server could not bind/connect; see artifacts/release-quality/poky/doctor.txt" >&2
  exit 1
fi
echo "fresh Poky local-clone workflow passed"
