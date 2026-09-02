# Current Task

## Task

**ID:** UX-SELECTION-VIEWPORT-002
**Title:** Keep every menu selection visible at the bottom edge
**Status:** DONE

## Objective

Every independently clipped menu must derive its visible rows from the current
stable selection and the actual rendered capacity.

## Dependencies

- UX-DOT-METERS-001 — DONE

## Definition of done

- The Compatibility capability inventory follows the selected stable identity
  through the last row.
- Viewport capacity accounts for the table border and header.
- The title reports the exact visible range and filtered inventory size.
- Full-inventory and narrow context-menu regressions prove the last highlighted
  row remains visible.
- Daemon IPC retries signal-interrupted reads so release validation and live
  sessions do not disconnect on a transient `EINTR`.
- PTY control responses allow the child termination deadline to complete under
  load instead of timing out first.
- Version 0.1.12 is installed and repository completion gates pass.

## Verification

```bash
cargo test -p yoctui-ui tests::ux_menu_renders_groups_context_disabled_safety_and_accessible_responsive_states -- --exact
cargo test -p yoctui-ui tests::ux_scrollable_collection_matrix_keeps_the_last_highlighted_row_visible -- --exact
cargo test -p yoctui-ui
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

The model remains the owner of selection. The renderer recomputes the bounded
viewport after every selection, filter, inventory replacement, and resize. The
completion gate also exposed and now covers an interrupted IPC read that could
otherwise reset a healthy daemon connection, plus a PTY control deadline that
was shorter than the child termination operation it awaited.
