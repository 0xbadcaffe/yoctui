# Current Task

## Task

**ID:** UX-TEXTAREA-MODEL-001
**Title:** Extend the reducer-owned multiline editor model
**Status:** NOT_STARTED

## Objective

Extend the existing reducer-owned editor into a reusable bounded multiline
model with explicit modes, selection, history, search/replace, validation, and
safe save lifecycle state.

## Dependencies

- `UX-KEYMAP-MODEL-001` — DONE
- `UX-SCROLL-001` — DONE

## Relevant files

- reducer-owned popup and workspace editor state
- typed editor actions and app input mapping
- validation spans, diff/conflict, and atomic-save lifecycle projections
- property and Unicode boundary tests
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Multiline Unicode cursor, selection, word/line/page motion, line numbers, and
  wrap metadata remain reducer-owned and bounded.
- Normal, Insert, and Visual modes are explicit; bracketed paste and clipboard
  remain typed effects.
- Bounded undo/redo and search/replace preserve valid UTF-8 positions.
- Validation spans, diff preview, external conflicts, atomic save, and recovery
  after save failure are distinct typed states.
- Property tests cover large and adversarial edit sequences without panics or
  unbounded history.

## Verification

```bash
cargo test -p yoctui-model ux_textarea
cargo test -p yoctui-app ux_textarea
cargo test -p yoctui-model --features proptest ux_textarea
```
