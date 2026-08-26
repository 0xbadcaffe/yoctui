# Current Task

## Task

**ID:** UX-KEYMAP-UI-001
**Title:** Add discoverable keybinding preferences and report
**Status:** NOT_STARTED

## Objective

Make the effective keymap searchable, understandable, safely editable,
resettable, and exportable from the real Settings workspace.

## Dependencies

- `UX-KEYMAP-MODEL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/keymap.rs`
- Settings reducer state in `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Render searchable effective bindings grouped by exact scope and action metadata.
- Distinguish default, custom, pending capture, conflict, and disabled states in text.
- Add focus-trapped capture/edit, remove, per-action reset, and reset-all controls.
- Refuse invalid/conflicting/unreachable edits with the model's exact reason.
- Export the deterministic effective-keymap report through a bounded typed effect.
- Persist successful edits atomically and retain retryable dirty state on failure.

## Verification

```bash
cargo test -p yoctui-ui ux_keymap_preferences
cargo test -p yoctui-app ux_keymap_preferences
cargo test -p yoctui -- ux_keymap_persistence
./scripts/verify-roadmap.sh
```

This task presents and edits the existing model. It does not introduce the F10
application/context menus owned by `UX-MENU-001`.
