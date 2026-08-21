#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
source_poky="${YOCTUI_POKY_SOURCE:-/home/$USER/src/poky}"
if [[ ! -x "$source_poky/oe-init-build-env" ]]; then
  printf 'live project profile: Poky source is unavailable: %s\n' "$source_poky" >&2
  exit 2
fi

work_root="$(mktemp -d "$repo_root/.yoctui-profile-poky.XXXXXX")"
trap 'rm -rf "$work_root"' EXIT
git clone --local --no-hardlinks "$source_poky" "$work_root/poky" >/dev/null
build_dir="$work_root/build"
mkdir -p "$build_dir"

set +u
source "$work_root/poky/oe-init-build-env" "$build_dir" >/dev/null
set -u
unset BBPATH
export OEROOT="$work_root/poky"
hosttool_note="none"
if ! command -v lz4c >/dev/null 2>&1; then
  printf '\nHOSTTOOLS:remove = "lz4c"\n' >>"$build_dir/conf/local.conf"
  hosttool_note="lz4c unavailable; removed from the isolated metadata-only HOSTTOOLS check"
fi
userns_note="none"
if ! unshare -Ur true >/dev/null 2>&1; then
  printf '\nINHERIT:remove = "sanity"\n' >>"$build_dir/conf/local.conf"
  userns_note="user namespaces unavailable; isolated metadata-only sanity hook disabled"
fi

cargo build -p yoctui >/dev/null
binary="$repo_root/target/debug/yoctui"
artifact_dir="$repo_root/artifacts/project-profile"
mkdir -p "$artifact_dir"

bitbake --kill-server >/dev/null 2>&1 || true
"$binary" --backend bridge --build-dir "$build_dir" profile >"$artifact_dir/no-profile.txt"
grep -Fq 'project profile: absent (optional)' "$artifact_dir/no-profile.txt"
grep -Fq 'BitBake version:' "$artifact_dir/no-profile.txt"

mkdir -p "$work_root/poky/.yoctui"
printf '%s\n' \
  'schema_version = 1' \
  '' \
  '[favorites]' \
  'recipes = ["base-files"]' \
  'images = ["core-image-minimal"]' \
  'layers = ["core"]' \
  '' \
  '[[build_presets]]' \
  'name = "minimal"' \
  'targets = ["core-image-minimal"]' \
  '' \
  '[build_presets.options]' \
  'continue_on_error = false' \
  '' \
  '[[workflows]]' \
  'name = "refresh"' \
  '' \
  '[[workflows.steps]]' \
  'type = "refresh_metadata"' \
  >"$work_root/poky/.yoctui/project.toml"

"$binary" --backend bridge --build-dir "$build_dir" profile >"$artifact_dir/with-profile.txt"
grep -Fq 'project profile: loaded' "$artifact_dir/with-profile.txt"
test "$(grep -Fc 'profile item: resolved' "$artifact_dir/with-profile.txt")" -eq 5
if grep -Eq 'profile item: (stale|ambiguous|unavailable)' "$artifact_dir/with-profile.txt"; then
  printf '%s\n' 'live project profile: a Poky identity did not resolve authoritatively' >&2
  exit 1
fi

{
  printf 'poky_revision=%s\n' "$(git -C "$work_root/poky" rev-parse HEAD)"
  printf 'poky_branch=%s\n' "$(git -C "$work_root/poky" branch --show-current)"
  printf 'hosttool_limitation=%s\n' "$hosttool_note"
  printf 'userns_limitation=%s\n' "$userns_note"
} >"$artifact_dir/workspace.txt"
printf '%s\n' 'live project profile: fresh Poky no-profile and with-profile paths passed'
