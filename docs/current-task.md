# Current Task

## Task

**ID:** UX-ACTION-CATALOG-001
**Title:** Unify all operator actions in one typed catalog
**Status:** NOT_STARTED

## Objective

Create the single model-owned action catalog that mechanically supplies menus,
the command palette, Help, footer hints, configured bindings, and availability
reasons.

## Dependencies

- `UX-SPEC-001` — DONE

## Relevant files

- `crates/yoctui-model/src/`
- `crates/yoctui-app/src/`
- `crates/yoctui-ui/src/`
- existing command-palette, Help, footer, and input action definitions
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Give every operator action a stable typed ID and canonical metadata.
- Record scope, menu path, label, description, aliases, palette keywords,
  default binding, requirements, safety class, footer priority, and Help group.
- Derive exact enabled/disabled state and reason from typed model capability and
  selection state.
- Project the same catalog into model, app, and UI consumers without parallel
  hard-coded action lists.
- Prove catalog identity, uniqueness, reachability, availability, and consumer
  parity with focused tests.

## Verification

```bash
cargo test -p yoctui-model ux_action_catalog
cargo test -p yoctui-app ux_action_catalog
cargo test -p yoctui-ui ux_action_catalog
./scripts/verify-roadmap.sh
```

This task establishes catalog authority. Configurable key chords and the full
menu rendering surface remain in their dependent tasks.
