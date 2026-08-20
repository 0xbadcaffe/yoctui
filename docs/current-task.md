# Current Task

## Task

**ID:** SEARCH-UI-001
**Title:** Improve search UX
**Status:** IN_PROGRESS

## Objective

Unify the visible search experience across existing typed metadata, log, and
command-palette searches so query text, focus, result counts,
next/previous navigation, and clearing are explicit and consistent.

## Dependencies

- `FOUNDATION-UI-003` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Search input uses one consistent visible presentation while retaining each
  workspace's existing typed query and reducer path.
- The current query, active-input focus, result count, next/previous controls,
  and clear action are explicit wherever supported.
- Empty query, no-match, one-match, many-match, and long-query states are
  bounded and unambiguous.
- Search focus never conflicts with pane, dialog, command-palette, or terminal
  focus; dialogs and the palette remain focus trapped.
- Keyboard behavior and footer/help labels agree with the typed keymap.
- Wide, medium, narrow, high-contrast, no-color, and reduced-motion rendering
  stays readable and panic-free.

## Verification

```bash
cargo test -p yoctui-ui next_generation_search
cargo test -p yoctui-app search
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
