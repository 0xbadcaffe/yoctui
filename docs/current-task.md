# Current Task

## Task

**ID:** DAEMON-UPGRADE-LIFECYCLE-001
**Title:** Preserve daemon identity across executable replacement
**Status:** DONE

## Objective

Keep lifecycle status and attach available when a release install atomically
replaces the on-disk Yoctui executable while the prior daemon image continues
owning active work. Linux's exact ` (deleted)` process-image suffix may match
the recorded executable; unrelated executable paths must remain foreign.

## Dependencies

- `DAEMON-ATTACH-QUIT-001` — DONE

## Relevant files

- `crates/yoctui-protocol/src/daemon_lifecycle.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Exact recorded paths and their Linux ` (deleted)` process images classify as
  the current daemon.
- Other executable paths still classify as foreign processes.
- The installed client reports the still-running daemon and build.
- Focused and baseline checks pass.
- Registry/status/current-task documentation is updated and committed.

All definition-of-done items and verification commands pass. This final
completed task remains the terminal handoff because every registry task is
`DONE`.

## Verification

```bash
cargo test -p yoctui-protocol daemon_lifecycle
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
