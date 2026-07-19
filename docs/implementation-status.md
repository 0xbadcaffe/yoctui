# Yoctui implementation status

Status values: `NOT_STARTED`, `IN_PROGRESS`, `BLOCKED`, `DONE`.

## Foundation and naming

- [DONE] Rust workspace exists and dependency direction is acyclic. Verification: `cargo metadata --no-deps`. Commit: `43ca39b`.
- [DONE] Remove every obsolete legacy application name from crate names, directories, imports, tests, paths, scripts, and history-facing checks. Verification: `./scripts/check-obsolete-name.sh`. Commit: `d2c38ad`.
- [DONE] Public binary, configuration directory, environment prefix, bridge name, and UI branding use Yoctui. Verification: `cargo run -p yoctui -- --help`. Commit: `ad603ad`.
- [NOT_STARTED] Add repository lint/format configuration and a fresh-clone setup check. Verification: CI and `./scripts/verify-completion.sh`.

## Application and terminal

- [DONE] Model, typed actions, pure reducer, bounded logs, task state, and basic TUI screens exist. Verification: `cargo test -p yoctui-model -p yoctui-ui`. Commit: multiple pre-guide commits.
- [DONE] Terminal guard restores raw mode, alternate screen, cursor, mouse, bracketed paste, and panic state. Verification: Rust tests and manual pseudo-terminal test. Commit: `7d50f94`.
- [IN_PROGRESS] Handle resize, supported termination signals, terminal restoration in a pseudo-terminal, and dynamic unavailable-command help. Verification: `./scripts/test-terminal.sh` and `cargo test -p yoctui`. Pseudo-terminal and SIGTERM coverage commits: `df411ad`; current signal commit pending.
- [IN_PROGRESS] Complete dashboard metrics, build dialog, confirmations, notifications, and backend-driven TUI effects. Verification: UI tests and fake-backend integration tests.

## Process backend

- [DONE] Process output capture, ANSI stripping, severity classification, process-group cancellation, escalation, invalid UTF-8 handling, and fake-process tests exist. Verification: `cargo test -p yoctui-bitbake`. Commit: `c477b6a`, `6c53488`, `491db9f`.
- [IN_PROGRESS] Bound individual process lines, preserve multiline diagnostics, map exit status, test forced cleanup/child trees/high-volume output, and expose cancellation outcome. Verification: process integration tests. Bounded-line commit pending.

## Protocol and bridge

- [DONE] Versioned envelopes, sequence/correlation fields, NDJSON framing, bounded transport reads, malformed/oversized handling, and Python framing tests exist. Verification: protocol and bridge tests. Commit: `3730f35`, `78cc988`.
- [IN_PROGRESS] Implement bridge handshake negotiation, graceful shutdown command, compatibility adapters, mocked BitBake integration boundary, and typed workspace/recipe/layer/variable responses. Verification: pytest with mocked modules.
- [NOT_STARTED] Connect bridge to a supported live BitBake server, normalize native events, start builds, request native cancellation, and document tested BitBake versions. Verification: optional real-Yocto smoke workflow.

## Workspace, CLI, and configuration

- [DONE] CLI options, configuration precedence, headless inspection, doctor diagnostics, and read-only backend CLI commands exist. Verification: CLI tests and `yoctui doctor`. Commit: `e033f62`, `35fa2cb`, `1979825`.
- [IN_PROGRESS] Complete workspace fields, recipe/layer discovery, variable provenance, CLI subcommand outputs, editor configuration, session persistence, and all configuration settings. Verification: fake bridge and CLI integration tests.

## Screens and interaction

- [IN_PROGRESS] Logs support bounded retention, pause/follow, wrap, severity filtering, and notification display. Verification: model/UI tests. Commit: `26aad33`, `4017e02`, `c871b26`.
- [NOT_STARTED] Add log scrolling, horizontal navigation, text search, recipe/task filters in UI, source-log/editor actions, and eviction detail.
- [NOT_STARTED] Complete structured errors screen with selection/detail/log jump.
- [NOT_STARTED] Complete recipes screen with search/details/valid actions and destructive confirmations.
- [NOT_STARTED] Complete layers screen with metadata/search/open action.
- [NOT_STARTED] Complete read-only configuration screen with search, expansion, and provenance.

## Reliability, testing, and quality

- [IN_PROGRESS] Expand model/protocol/UI/process tests; add fake bridge fixtures and integration test tree. Verification: `cargo test --workspace --all-features`, `pytest`.
- [NOT_STARTED] Add property tests, fuzz targets, stress/memory retention tests, benchmarks, and terminal integration tests.
- [NOT_STARTED] Configure coverage (`cargo llvm-cov`, `pytest-cov`) with thresholds.
- [NOT_STARTED] Configure audit/deny/ruff/mypy checks and complete CI matrix, optional real-Yocto, sanitizer, Valgrind, and flamegraph workflows.
- [NOT_STARTED] Run deterministic Valgrind, profiling, flamegraph, and memory workloads; commit concise reports.
- [IN_PROGRESS] Complete all documentation and compatibility matrix.
- [NOT_STARTED] Add `scripts/verify-completion.sh`, artifacts directories, and completion-gate checks.

## CONTINUE_FROM_HERE

Current phase: process-backend reliability.

Next incomplete item: preserve multiline diagnostics and map process exit outcomes.

Relevant files: `crates/yoctui-bitbake/src/lib.rs`, `docs/implementation-status.md`.

Last successful commands: `cargo fmt --all --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

Next command: add a process-result model and test failed exit handling.
