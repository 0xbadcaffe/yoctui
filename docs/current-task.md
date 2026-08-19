# Current Task

## Task

**ID:** LOG-UI-002
**Title:** Add compact log activity indicator
**Status:** IN_PROGRESS

## Objective

Provide one compact textual activity projection that consistently exposes the
live log mode without requiring the full Logs status panel.

## Dependencies

- `LOG-UI-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Following and paused are textually distinct.
- Active filters and search are visible without expanding the status panel.
- Any retention eviction is visible and includes warning/error loss where room
  permits.
- The compact form remains readable in embedded and narrow contexts.
- No state relies only on color or animation.

## Verification

```bash
cargo test -p yoctui-ui next_generation_log_activity
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
