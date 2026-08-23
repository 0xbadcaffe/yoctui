#!/usr/bin/env bash
set -euo pipefail

repo_root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo_root"

reference=docs/reference/bitbake-cheatsheet-wrynose-6.0-bitbake-2.18.md
expected=ad95ecfa6a17691fa2a6d12f598f01fbd33de524c2a08ebccd218ef5fe88dd47
actual=$(sha256sum "$reference")
actual=${actual%% *}

if [[ "$actual" != "$expected" ]]; then
    echo "Raw reference SHA-256 mismatch: expected $expected, got $actual" >&2
    exit 1
fi

python3 scripts/generate-raw-catalog.py --check
cargo test -p yoctui-model raw_catalog_trace

echo "Raw catalog traceability valid: 32 categories, 464 commands, 288 executable"
