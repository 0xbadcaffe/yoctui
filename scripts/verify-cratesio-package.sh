#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

version="0.1.68"
public_crates=(
  yoctui-model
  yoctui-protocol
  yoctui-bitbake
  yoctui-app
  yoctui-ui
  yoctui
)
work_dir="$(mktemp -d)"
package_target_dir="$repo_root/target/cratesio-package"
trap 'rm -rf "$work_dir"' EXIT

for private_manifest in crates/yoctui-e2e/Cargo.toml crates/yoctui-shell/Cargo.toml; do
  if ! grep -Eq '^publish = false$' "$private_manifest"; then
    printf 'private package is not protected from publication: %s\n' "$private_manifest" >&2
    exit 1
  fi
done

package_args=()
for crate_name in "${public_crates[@]}"; do
  package_args+=(--package "$crate_name")
done
cargo package "${package_args[@]}" --allow-dirty --no-verify

for crate_name in "${public_crates[@]}"; do
  archive="target/package/${crate_name}-${version}.crate"
  if [[ ! -s "$archive" ]]; then
    printf 'missing package archive: %s\n' "$archive" >&2
    exit 1
  fi
  archive_size="$(stat -c '%s' "$archive")"
  if (( archive_size > 10485760 )); then
    printf 'package exceeds crates.io 10 MiB limit: %s (%s bytes)\n' \
      "$crate_name" "$archive_size" >&2
    exit 1
  fi
  if tar -tzf "$archive" | grep -Eq '(^|/)(target|artifacts|\.git)/'; then
    printf 'package contains a forbidden build or repository artifact: %s\n' "$crate_name" >&2
    exit 1
  fi
  tar -xzf "$archive" -C "$work_dir"
done

if ! tar -tzf "target/package/yoctui-bitbake-${version}.crate" \
  | grep -E '/bridge/yoctui_bridge.py$' >/dev/null; then
  printf '%s\n' 'yoctui-bitbake package does not contain the bundled bridge source' >&2
  exit 1
fi

{
  printf '%s\n' '[workspace]'
  printf '%s\n' 'resolver = "2"'
  printf '%s\n' 'members = ['
  for crate_name in "${public_crates[@]}"; do
    printf '  "%s-%s",\n' "$crate_name" "$version"
  done
  printf '%s\n' ']'
  printf '%s\n' '[patch.crates-io]'
  for crate_name in "${public_crates[@]}"; do
    printf '%s = { path = "%s-%s" }\n' "$crate_name" "$crate_name" "$version"
  done
} >"$work_dir/Cargo.toml"

CARGO_TARGET_DIR="$package_target_dir" \
  cargo check --manifest-path "$work_dir/Cargo.toml" --workspace --all-features

config_dir="$work_dir/config"
mkdir -p "$config_dir" "$work_dir/build" "$work_dir/runtime" "$work_dir/state"
chmod 700 "$work_dir/runtime"
version_output="$(
  XDG_CONFIG_HOME="$config_dir" CARGO_TARGET_DIR="$package_target_dir" cargo run \
    --manifest-path "$work_dir/Cargo.toml" -p yoctui --quiet -- --version
)"
if [[ "$version_output" != "yoctui ${version}" ]]; then
  printf 'unexpected packaged binary version: %s\n' "$version_output" >&2
  exit 1
fi

help_output="$(
  XDG_CONFIG_HOME="$config_dir" CARGO_TARGET_DIR="$package_target_dir" cargo run \
    --manifest-path "$work_dir/Cargo.toml" -p yoctui --quiet -- --help
)"
if [[ "$help_output" != *"Ratatui frontend and control client for BitBake"* ]]; then
  printf '%s\n' 'packaged binary help output is incomplete' >&2
  exit 1
fi

# An empty directory cannot provide daemon-owned BitBake authority. Exercise
# the packaged bridge handshake/shutdown instead, outside the source checkout
# and without inheriting a user's daemon or external bridge override.
headless_output="$(
  cd "$work_dir"
  env -u YOCTUI_BRIDGE_PATH -u BUILDDIR -u BBPATH -u BBSERVER \
    -u PYTHONPATH -u TEMPLATECONF \
    XDG_CONFIG_HOME="$config_dir" XDG_RUNTIME_DIR="$work_dir/runtime" \
    XDG_STATE_HOME="$work_dir/state" \
    "$package_target_dir/debug/yoctui" --build-dir "$work_dir/build" doctor
)"
if [[ "$headless_output" != *"bridge protocol: ok (bounded handshake and shutdown)"* ]]; then
  printf '%s\n' "$headless_output" >&2
  printf '%s\n' 'packaged binary could not run its bundled bridge' >&2
  exit 1
fi
if [[ "$headless_output" != *"authority: Unavailable"* ]]; then
  printf '%s\n' 'package diagnostic unexpectedly acquired daemon authority' >&2
  exit 1
fi

printf 'crates.io package graph verified: %s\n' "${public_crates[*]}"
