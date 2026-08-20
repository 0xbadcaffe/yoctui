# Current Task

## Task

**ID:** FOOTER-UI-002
**Title:** Add transient status area
**Status:** IN_PROGRESS

## Objective

Add a bounded transient status area to the persistent footer that presents
typed notifications, operation outcomes, errors, confirmations, reconnects,
and background activity without hiding critical contextual shortcuts.

## Dependencies

- `FOOTER-UI-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Notifications, operation results, errors, pending confirmations, daemon
  reconnect state, and background activity use typed existing model state.
- Status priority is deterministic: error and pending confirmation precede
  notification/result, reconnect, and background activity.
- Status text is bounded and never pushes critical context shortcuts outside
  their measured footer region.
- No result, progress, reconnect, or activity is inferred from absent or stale
  state.
- Text markers preserve severity in high-contrast and no-color modes;
  reduced-motion does not change semantic content.
- Wide, medium, narrow, empty, long-text, and minimum-width layouts remain
  readable and panic-free.

## Verification

```bash
cargo test -p yoctui-ui next_generation_transient_status
cargo test -p yoctui-model notification
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
