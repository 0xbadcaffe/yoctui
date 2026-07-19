#!/usr/bin/env bash
set -euo pipefail
cargo build --release -p ratabake
echo "Run: target/release/ratabake --headless --backend process <target>"
