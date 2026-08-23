# Current Task

## Task

**ID:** RAW-OUTPUT-UI-001
**Title:** Implement Raw execution output workspace
**Status:** IN_PROGRESS

## Objective

Render daemon-owned Raw job and interactive PTY execution state as the typed
Raw Mode output workspace, including lifecycle, bounded output or terminal
session, follow/search/scroll controls, cancellation, detach/reattach, and
terminal result identity.

## Dependencies

- `RAW-JOB-001` — DONE
- `RAW-PTY-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-app/src/pty_context.rs`
- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-cli/src/client_runtime.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Raw Mode opens a typed execution workspace for the selected daemon replica
  and shows exact command identity, interaction kind, lifecycle, attachment,
  elapsed time, terminal outcome, exit code, and bounded drop/truncation state.
- Noninteractive jobs render independently identified stdout/stderr with
  follow/pause, search, vertical scrolling, and safe horizontal scrolling.
- Interactive executions render and attach through the existing daemon PTY
  terminal/session components; UI widgets do not parse terminal bytes.
- Cancellation and termination remain explicit typed actions, while closing or
  detaching the view never terminates daemon-owned work.
- Reattach/reconnect replaces the workspace from current validated Raw and PTY
  replicas, including terminal `Lost` state, without inventing local ownership.
- Keyboard, mouse, narrow-terminal, focus, and `TestBackend` coverage matches
  the authoritative Raw Mode specification without adding new layout behavior.

## Verification

```bash
cargo test -p yoctui-model raw_output
cargo test -p yoctui-app raw_output
cargo test -p yoctui-ui raw_output
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
