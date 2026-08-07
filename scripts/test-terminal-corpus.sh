#!/usr/bin/env bash
set -euo pipefail
cargo test -p yoctui-shell terminal_emulation
echo "terminal corpus passed"
