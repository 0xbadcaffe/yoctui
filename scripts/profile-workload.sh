#!/usr/bin/env bash
set -euo pipefail
cargo build --release -p yoctui
echo "Run: target/release/yoctui --headless --backend process <target>"
