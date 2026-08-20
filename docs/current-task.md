# Current Task

## Task

**ID:** DIALOG-UI-001
**Title:** Polish typed dialogs
**Status:** IN_PROGRESS

## Objective

Unify every typed dialog around one documented visual structure for title,
body, aligned fields, selection, disabled state, buttons, keyboard hints, and
validation while retaining typed focus and confirmation semantics.

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

- Shared dialog primitives provide consistent border, title, body, aligned
  field, selected-control, disabled-control, button, hint, and validation
  presentation without erasing typed dialog identity.
- Dialogs remain modal and focus trapped; closing restores the exact previous
  pane and destructive confirmation behavior does not weaken.
- Enabled and disabled actions are textually distinct, and disabled reasons
  remain discoverable without relying on color.
- Validation errors have a stable bounded area and do not displace or clip
  confirmation controls.
- Wide, medium, narrow, minimum, long-content, high-contrast, no-color, and
  reduced-motion dialog states remain useful and panic-free.
- Existing typed workflows and input mappings remain intact; no one-off shell
  execution or rendering-owned state mutation is introduced.

## Verification

```bash
cargo test -p yoctui-ui next_generation_dialogs
cargo test -p yoctui-model dialog
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
