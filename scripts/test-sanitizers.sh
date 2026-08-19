#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

sanitizer_target="x86_64-unknown-linux-gnu"
if [[ "$(uname -s)" != "Linux" || "$(uname -m)" != "x86_64" ]]; then
  printf '%s\n' 'Yoctui sanitizer verification currently requires Linux x86_64' >&2
  exit 2
fi
if ! rustup toolchain list | grep -q '^nightly'; then
  printf '%s\n' 'nightly Rust is required; install it with rustup toolchain install nightly' >&2
  exit 2
fi
nightly_sysroot="$(rustc +nightly --print sysroot)"
if [[ ! -f "$nightly_sysroot/lib/rustlib/src/rust/library/Cargo.toml" ]]; then
  printf '%s\n' 'nightly rust-src is required; install it with rustup component add rust-src --toolchain nightly' >&2
  exit 2
fi

printf '%s\n' 'running deterministic stress workloads under AddressSanitizer'
CARGO_TARGET_DIR=target/sanitizers/address \
RUSTFLAGS='-Zsanitizer=address -Cdebuginfo=1' \
ASAN_OPTIONS='detect_leaks=0:halt_on_error=1' \
cargo +nightly test \
  -Zbuild-std=std,panic_abort \
  --target "$sanitizer_target" \
  -p yoctui-model \
  -p yoctui-protocol \
  hardening_stress

printf '%s\n' 'running the production workbench workload under AddressSanitizer'
profile_binary="$(
  CARGO_TARGET_DIR=target/sanitizers/address \
  RUSTFLAGS='-Zsanitizer=address -Cdebuginfo=1' \
  ASAN_OPTIONS='detect_leaks=0:halt_on_error=1' \
  cargo +nightly build \
    -Zbuild-std=std,panic_abort \
    --target "$sanitizer_target" \
    -p yoctui \
    --bench workbench_profile \
    --message-format=json | \
    python3 -c 'import json, sys
for line in sys.stdin:
    event = json.loads(line)
    target = event.get("target", {})
    if event.get("reason") == "compiler-artifact" and target.get("name") == "workbench_profile" and event.get("executable"):
        print(event["executable"])
'
)"
if [[ ! -x "$profile_binary" ]]; then
  printf '%s\n' 'AddressSanitizer workbench benchmark executable was not produced' >&2
  exit 1
fi
sanitizer_workload="$(mktemp)"
trap 'rm -f "$sanitizer_workload"' EXIT
ASAN_OPTIONS='detect_leaks=0:halt_on_error=1' \
YOCTUI_PROFILE_FRAMES=128 \
  "$profile_binary" >"$sanitizer_workload" 2>&1
if ! grep -Eq '^yoctui workbench profile: frames=128 checksum=[0-9a-f]{16} elapsed_ms=[1-9][0-9]*$' "$sanitizer_workload"; then
  printf '%s\n' 'AddressSanitizer workbench benchmark did not complete' >&2
  exit 1
fi

printf '%s\n' 'running deterministic stress workloads under LeakSanitizer'
CARGO_TARGET_DIR=target/sanitizers/leak \
RUSTFLAGS='-Zsanitizer=leak -Cdebuginfo=1' \
LSAN_OPTIONS='exitcode=23' \
cargo +nightly test \
  -Zbuild-std=std,panic_abort \
  --target "$sanitizer_target" \
  -p yoctui-model \
  -p yoctui-protocol \
  hardening_stress
