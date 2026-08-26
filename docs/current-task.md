# Current Task

## Task

**ID:** UX-TEXTAREA-UI-001
**Title:** Render safe multiline editors and decide ratatui-textarea adoption
**Status:** NOT_STARTED

## Objective

Render the reducer-owned multiline editor state across responsive layouts and
decide the audited `ratatui-textarea` adapter candidate with feature-parity
evidence and no second mutable state authority.

## Dependencies

- `UX-TEXTAREA-MODEL-001` — DONE
- `UX-WIDGET-PRIMITIVES-001` — DONE
- `UX-LICENSE-001` — DONE

## Relevant files

- popup and workspace editor renderers in `yoctui-ui` and `yoctui-cli`
- model-to-render projection and input/mouse adapter seams
- editor concept screen and responsive/no-color/ASCII goldens
- dependency decision and third-party compliance evidence
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Multiline text, line numbers, cursor/selection, mode, validation, search,
  diff/save state, and shortcut guidance render from `TextAreaState` only.
- Keyboard, bracketed paste, clipboard, and mouse selection map to typed model
  actions without retaining widget-owned authority.
- Wide, medium, narrow, below-minimum, Unicode, ASCII, and no-color layouts are
  deterministic and preserve validation/save meaning.
- The `ratatui-textarea` adapter either round-trips complete model state or is
  rejected with tested custom-renderer feature-parity evidence.

## Verification

```bash
cargo test -p yoctui-ui ux_textarea
cargo test -p yoctui -- ux_textarea
cargo deny check
```
