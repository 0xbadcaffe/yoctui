# Current Task

## Task

**ID:** UX-LIST-TREE-001
**Title:** Evaluate and integrate tree scrollview and variable-list adapters
**Status:** NOT_STARTED

## Objective

Evaluate tree, scrollview, and variable-list adapters against Yoctui's external
model authority and integrate bounded render-only list/tree projections.

## Dependencies

- `UX-KEYMAP-MODEL-001` — DONE
- `UX-WIDGET-PRIMITIVES-001` — DONE
- `UX-LICENSE-001` — DONE

## Relevant files

- shared checkbox model and reducer actions
- app keyboard and mouse mapping
- render-only checkbox primitive and semantic text fallbacks
- batch preview/confirmation workflows
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Checked, unchecked, indeterminate, disabled, and focused states are typed and
  retain text/ASCII meaning without color.
- Space toggles an enabled selection; Enter retains the workflow primary action
  and selection alone never executes work.
- Batch actions resolve and preview every exact target before the existing
  confirmation boundary, including empty/partial/disabled inputs.
- Keyboard, mouse, responsive, no-color, and bounded property tests pass.

## Verification

```bash
cargo test -p yoctui-model ux_checkbox
cargo test -p yoctui-app ux_checkbox
cargo test -p yoctui-ui ux_checkbox
```
