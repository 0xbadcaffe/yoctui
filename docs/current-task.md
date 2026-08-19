# Current Task

## Task

**ID:** INSPECTOR-UI-003
**Title:** Add action list presentation
**Status:** IN_PROGRESS

## Objective

Present contextual Inspector actions with aligned names and authoritative
shortcuts, availability, and disabled reasons across responsive layouts.

## Dependencies

- `INSPECTOR-UI-001` — DONE

## Relevant files

- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Contextual actions render in an aligned `Action Name / Shortcut` grammar.
- Enabled and disabled states remain distinguishable without relying on color.
- Discoverable disabled actions retain their exact typed reason in the
  Inspector or help path.
- Only authoritative keymap bindings are displayed.
- Wide, overlay, narrow, high-contrast, and no-color layouts remain safe.

## Verification

```bash
cargo test -p yoctui-ui next_generation_inspector_actions
cargo test -p yoctui-app compatibility_ui_actions
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
