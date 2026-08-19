# Current Task

## Task

**ID:** NAV-UI-001
**Title:** Redesign Navigator presentation
**Status:** IN_PROGRESS

## Objective

Refine the Navigator into a polished, deterministic workspace rail with
terminal-safe hierarchy, authoritative contextual rows, bounded scrolling,
and complete keyboard and mouse selection.

## Dependencies

- `FOUNDATION-UI-003` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Workspace destinations remain visually grouped and deterministically ordered.
- Active selection, focus, badges, and optional contextual entries use only
  authoritative typed state.
- Workspace navigation remains distinct from workspace-owned content trees.
- Keyboard and mouse selection share typed action routing.
- Scrolling is bounded and stable at all supported widths.

## Verification

```bash
cargo test -p yoctui-ui next_generation_navigator
cargo test -p yoctui-app next_generation_navigator
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
