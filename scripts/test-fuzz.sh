#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

if ! rustup toolchain list | grep -q '^nightly'; then
  printf '%s\n' 'nightly Rust is required; install it with rustup toolchain install nightly' >&2
  exit 2
fi
if ! cargo +nightly fuzz --version >/dev/null 2>&1; then
  printf '%s\n' 'cargo-fuzz is required; install it with cargo install cargo-fuzz' >&2
  exit 2
fi

mkdir -p artifacts/fuzz/protocol_frames artifacts/fuzz/retained_logs
fuzz_work_dir="$(mktemp -d)"
trap 'rm -rf "$fuzz_work_dir"' EXIT
cargo fmt --manifest-path fuzz/Cargo.toml -- --check
for target in protocol_frames retained_logs; do
  cp -R "fuzz/corpus/$target" "$fuzz_work_dir/$target"
  ASAN_OPTIONS=detect_leaks=0 cargo +nightly fuzz run "$target" "$fuzz_work_dir/$target" -- \
    -runs=64 \
    -max_len=4096 \
    -timeout=5 \
    -artifact_prefix="artifacts/fuzz/$target/"
done
