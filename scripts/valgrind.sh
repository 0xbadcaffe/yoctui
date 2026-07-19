#!/usr/bin/env bash
set -euo pipefail
mkdir -p artifacts/valgrind
cargo build -p yoctui
valgrind --leak-check=full --show-leak-kinds=all --track-fds=yes --track-origins=yes --error-exitcode=1 --xml=yes --xml-file=artifacts/valgrind/report.xml target/debug/yoctui --headless --backend process 2>&1 | tee artifacts/valgrind/summary.txt
