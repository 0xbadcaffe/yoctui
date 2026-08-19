# Current Task

## Task

**ID:** FOUNDATION-UI-002
**Title:** Create reusable layout primitives
**Status:** IN_PROGRESS

## Objective

Implement reusable render-only primitives for the documented workbench shell
and state presentation before migrating individual workspaces.

## Dependencies

- `FOUNDATION-UI-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Pane shell, section header, focused/unfocused borders, selected rows, and
  separators are reusable.
- Status labels and empty, unavailable, and loading states share typed
  presentation helpers.
- Bounded scroll indicators and responsive column selection are reusable.
- Primitive tests cover color, no-color, focus, bounds, and narrow geometry.

## Verification

```bash
cargo test -p yoctui-ui foundation_ui_primitives
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
