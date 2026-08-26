# Current Task

## Task

**ID:** UX-KEYMAP-MODEL-001
**Title:** Implement scoped configurable keybinding model
**Status:** NOT_STARTED

## Objective

Create the versioned model-owned keymap that binds configurable key sequences
to stable operator action IDs without collisions or unreachable critical
actions.

## Dependencies

- `UX-ACTION-CATALOG-001` — DONE

## Relevant files

- `crates/yoctui-model/src/action_catalog.rs`
- new keymap model and reducer state
- `crates/yoctui-app/src/lib.rs`
- session preference persistence and migration
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Define bounded validated single-key and multi-key chord sequences.
- Scope bindings globally or to exact action contexts using stable catalog IDs.
- Preserve documented defaults and aliases while allowing explicit overrides.
- Reject active same-scope collisions, reserved terminal-prefix conflicts,
  invalid sequences, and removal of the last route to a critical action.
- Persist atomically with schema versioning and migrate legacy preferences.
- Project one effective keymap for app input routing and later UI consumers.

## Verification

```bash
cargo test -p yoctui-model ux_keymap
cargo test -p yoctui-app ux_keymap
cargo test -p yoctui -- ux_keymap
./scripts/verify-roadmap.sh
```

The keymap must not weaken dialog/editor/terminal focus traps. Discoverable
editing UI and F10 menu rendering remain in their dependent tasks.
