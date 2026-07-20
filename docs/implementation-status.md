# Yoctui implementation status

Status values: `NOT_STARTED`, `IN_PROGRESS`, `BLOCKED`, `DONE`.

## Foundation and naming

- [DONE] Rust workspace exists and dependency direction is acyclic. Verification: `cargo metadata --no-deps`. Commit: `43ca39b`.
- [DONE] Remove every obsolete legacy application name from crate names, directories, imports, tests, paths, scripts, and history-facing checks. Verification: `./scripts/check-obsolete-name.sh`. Commit: `d2c38ad`.
- [DONE] Public binary, configuration directory, environment prefix, bridge name, and UI branding use Yoctui. Verification: `cargo run -p yoctui -- --help`. Commit: `ad603ad`.
- [IN_PROGRESS] Add repository lint/format configuration and a fresh-clone setup check. Verification: `./scripts/check-checkout.sh`; editor configuration and hidden-path naming guard: `1eab5bf`. Final completion gate remains pending.

## Application and terminal

- [DONE] Model, typed actions, pure reducer, bounded logs, task state, and basic TUI screens exist. Verification: `cargo test -p yoctui-model -p yoctui-ui`. Commit: multiple pre-guide commits.
- [DONE] Terminal guard restores raw mode, alternate screen, cursor, mouse, bracketed paste, and panic state. Verification: Rust tests and manual pseudo-terminal test. Commit: `7d50f94`.
- [IN_PROGRESS] Handle resize, supported termination signals, terminal restoration in a pseudo-terminal, and dynamic unavailable-command help. Verification: `./scripts/test-terminal.sh` and `cargo test -p yoctui`. Pseudo-terminal and SIGTERM coverage commits: `df411ad`; current signal commit pending.
- [IN_PROGRESS] Complete dashboard metrics, build dialog, confirmations, notifications, and backend-driven TUI effects. Verification: UI tests and fake-backend integration tests.

## Process backend

- [DONE] Process output capture, ANSI stripping, severity classification, process-group cancellation, escalation, invalid UTF-8 handling, and fake-process tests exist. Verification: `cargo test -p yoctui-bitbake`. Commit: `c477b6a`, `6c53488`, `491db9f`.
- [IN_PROGRESS] Bound individual process lines, preserve multiline diagnostics, map exit status, test forced cleanup/child trees/high-volume output, and expose cancellation outcome. Verification: process integration tests. Bounded lines: `ecbe553`; exit-code commit pending.

## Protocol and bridge

- [DONE] Versioned envelopes, sequence/correlation fields, NDJSON framing, bounded transport reads, malformed/oversized handling, and Python framing tests exist. Verification: protocol and bridge tests. Commit: `3730f35`, `78cc988`.
- [IN_PROGRESS] Implement bridge handshake negotiation, graceful shutdown command, compatibility adapters, mocked BitBake integration boundary, and typed workspace/recipe/layer/variable responses. Verification: pytest with mocked modules and `cargo test -p yoctui-bitbake`. Handshake: `496b177`; shutdown acknowledgement and child exit: `ad52654`; typed responses: `0f4bb33`; adapter selection: `c5daf0b`; mocked event normalization: `da142a2`.
- [IN_PROGRESS] Connect bridge to a supported live BitBake server, normalize native events, start builds, request native cancellation, and document tested BitBake versions. Verification: mocked `bb.server` adapter tests; live-Yocto smoke workflow remains required. Server boundary and unavailable-server diagnostics: `993ac4c`.

## Workspace, CLI, and configuration

- [DONE] CLI options, configuration precedence, headless inspection, doctor diagnostics, and read-only backend CLI commands exist. Verification: CLI tests and `yoctui doctor`. Commit: `e033f62`, `35fa2cb`, `1979825`.
- [IN_PROGRESS] Complete workspace fields, recipe/layer discovery, variable provenance, CLI subcommand outputs, editor configuration, session persistence, and all configuration settings. Verification: fake bridge and CLI integration tests.

## Screens and interaction

- [IN_PROGRESS] Logs support bounded retention, pause/follow, wrap, severity filtering, notification display, vertical/horizontal navigation, and interactive text search. Verification: model/UI tests. Commits: `26aad33`, `4017e02`, `c871b26`, `7c04b37`, `09c4978`, `7928bbd`.
- [IN_PROGRESS] Add recipe/task filters in UI, source-log/editor actions, and richer eviction detail.
- [IN_PROGRESS] Complete structured errors screen with selection/detail/log jump. Table/detail: `bec99cf`; selection: `4ed019b`; log jump: `8f0154f`. Cross-screen context and richer parsing remain.
- [IN_PROGRESS] Complete recipes screen with search/details/valid actions and destructive confirmations. Backend-loaded table, selection, and details: `58c9332`; search/actions/confirmations remain.
- [IN_PROGRESS] Complete layers screen with metadata/search/open action. Backend-loaded table, selection, and metadata details: `63895c7`; search/open action remain.
- [NOT_STARTED] Complete read-only configuration screen with search, expansion, and provenance.

## Reliability, testing, and quality

- [IN_PROGRESS] Expand model/protocol/UI/process tests; add fake bridge fixtures and integration test tree. Verification: `cargo test --workspace --all-features`, `pytest`, `./scripts/test-cli.sh`. Property tests: `b231871`, `36374dd`; bridge CLI smoke: `a1723c2`.
- [IN_PROGRESS] Add property tests, fuzz targets, stress/memory retention tests, benchmarks, and terminal integration tests. Retention and protocol framing properties complete; fuzz, stress, benchmarks remain.
- [DONE] Configure coverage (`cargo llvm-cov`, `pytest-cov`) with thresholds. Model/protocol Rust thresholds: `9c2ca20`; bridge Python coverage is 81.36%: `15ebffd`.
- [IN_PROGRESS] Configure audit/deny/ruff/mypy checks and complete CI matrix, optional real-Yocto, sanitizer, Valgrind, and flamegraph workflows. Ruff/mypy/pytest are enabled locally and in CI: pending commit; audit/deny and remaining workflows remain.
- [IN_PROGRESS] Run deterministic Valgrind, profiling, flamegraph, and memory workloads; commit concise reports. Reproducible bridge workload: `ed9ea9b`; release and Valgrind baselines: `664c36e`; Flamegraph remains pending.
- [IN_PROGRESS] Complete all documentation and compatibility matrix.
- [IN_PROGRESS] Add `scripts/verify-completion.sh`, artifacts directories, and completion-gate checks. Strict gate and artifact root: `2a623ef`; required coverage/audit/static-analysis tools and remaining product checks still prevent a passing final gate.

## CONTINUE_FROM_HERE

Current phase: bridge reliability and compatibility.

Next incomplete item: validate the live server adapter against a supported BitBake interface and normalize its native events.

Relevant files: `crates/yoctui-bitbake/src/lib.rs`, `bridge/yoctui_bridge.py`, `bridge/tests/test_bridge.py`, `docs/implementation-status.md`.

Last successful commands: `cargo fmt --all --check`, `cargo test --workspace --all-features`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

Next command: add a fixture-backed native-event adapter and document the unsupported live-server limitations, then run `cargo test --workspace --all-features`.
