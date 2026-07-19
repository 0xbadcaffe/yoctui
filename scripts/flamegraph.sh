#!/usr/bin/env bash
set -euo pipefail
mkdir -p artifacts/flamegraph
cargo flamegraph --root --bin yoctui -- --headless --backend process
