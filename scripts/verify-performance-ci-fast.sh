#!/usr/bin/env bash
set -euo pipefail

repo_root="$(git rev-parse --show-toplevel)"
cd "$repo_root"

[[ -x target/debug/yoctui ]] || cargo build --locked -p yoctui

# Idle blocking and dirty rendering must stay cheap without a long benchmark.
python3 scripts/test-idle-event-loops.py --binary target/debug/yoctui --sample-seconds 10
cargo test -q -p yoctui --bin yoctui render_scheduler

# High-rate reducer paths must coalesce work while preserving terminal records.
cargo test -q -p yoctui-model task_batches_coalesce_progress_and_preserve_terminal_failures
cargo test -q -p yoctui-model high_volume_logs_remain_within_retention_limits
cargo test -q -p yoctui-protocol daemon_journal_updates_high_rate_job_progress_without_full_snapshot_serialization

# Exercise every available CPU and the bounded production IPC path.
./scripts/verify-saturation-responsiveness.sh --harness
./scripts/verify-ipc-continuity.sh --backpressure
python3 -m unittest scripts/test_build_performance_regression_record.py

printf '%s\n' 'fast performance CI passed'
