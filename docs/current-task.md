# Current Task

## Task

**ID:** PALETTE-UI-001
**Title:** Polish command palette
**Status:** IN_PROGRESS

## Objective

Polish the focus-trapped command palette into a responsive workbench overlay
that clearly presents command, shortcut, availability, description, shared
search state, and bounded scroll position.

## Dependencies

- `SEARCH-UI-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The overlay is centered at supported wide/medium sizes and safely fills the
  useful narrow viewport without clipping controls.
- Every visible row presents command, shortcut, and exact availability from
  the existing typed command catalog.
- Selection exposes description plus disabled/compatibility reason without
  hiding the result list.
- Shared search state, result count, and clear behavior remain visible.
- A bounded scroll indicator names the selected result position and visible
  window; no command list is duplicated in rendering code.
- Palette focus remains trapped until close or typed activation, with clear
  selection and disabled presentation in all themes and no-color mode.
- Empty, no-match, long-content, wide, medium, narrow, and minimum terminal
  states remain useful and panic-free.

## Verification

```bash
cargo test -p yoctui-ui next_generation_palette
cargo test -p yoctui-app command_palette
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
