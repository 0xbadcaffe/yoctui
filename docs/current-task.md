# Current Task

## Task

**ID:** RAW-HISTORY-001
**Title:** Persist bounded Raw command history
**Status:** IN_PROGRESS

## Objective

Retain a bounded, newest-first history of completed Raw executions using safe
template, parameter, timing, and terminal-result metadata without persisting
live process authority, unbounded output, or temporary daemon identities.

## Dependencies

- `RAW-EXEC-MODEL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-protocol/src/daemon.rs`
- `crates/yoctui-protocol/src/daemon_persist.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-cli/src/daemon_persist.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/architecture.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- A versioned bounded Raw history record retains stable command/template
  identity, sanitized typed parameter/default values, interaction mode,
  start/end timing, terminal outcome, exit code, and a safe durable reference.
- History never persists PID, process group, writer lease, executable path,
  capability authority, full stdout/stderr, PTY screen, secret, or temporary
  live job/session identity.
- Only validated terminal daemon replicas enter history; duplicate request
  identities update idempotently and ordering remains newest first.
- Safe daemon persistence bounds record count and aggregate bytes, rejects
  malformed/oversized/unknown-version data, and recovers valid records without
  resurrecting process ownership.
- Reopening history selects current catalog identity and compatibility; running
  again still requires a fresh form, exact preview, confirmation, and request.
- Model and CLI tests cover success/failure/cancel/loss, sanitization, bounds,
  duplicate replacement, catalog staleness, persistence, and recovery.

## Verification

```bash
cargo test -p yoctui-model raw_history
cargo test -p yoctui -- raw_history
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
