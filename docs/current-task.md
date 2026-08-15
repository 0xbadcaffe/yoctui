# Current Task

## Task

**ID:** DAEMON-ATTACH-QUIT-001
**Title:** Restore global quit after daemon attach
**Status:** IN_PROGRESS

## Objective

Restore the documented global `q` and `Ctrl+C` behavior when daemon attach
retains Workspace focus. The client must exit cleanly, restore the terminal,
detach only the UI, and leave the daemon-owned BitBake build running.

## Dependencies

- `DAEMON-ATTACH-BUILD-001` — DONE

## Relevant files

- `crates/yoctui-cli/src/main.rs`
- `crates/yoctui-app/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Workspace, Navigator, and Inspector focus all preserve global `q`/`Ctrl+C`.
- Dialogs and editors continue trapping their context-specific input.
- The real terminal lifecycle probe observes alternate-screen restoration.
- Exiting an attached client does not stop the active daemon build.
- Focused and baseline checks pass.
- Registry/status/current-task documentation is updated and committed.

## Verification

```bash
cargo test -p yoctui-app persistent_pane_focus_preserves_global_quit
./scripts/test-terminal.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
