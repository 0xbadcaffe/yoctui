# Current Task

## Task

**ID:** FOOTER-UI-001
**Title:** Redesign bottom shortcut bar
**Status:** IN_PROGRESS

## Objective

Redesign the persistent shortcut rail so every displayed action is relevant to
the current context, backed by the authoritative typed keymap, and readable at
all supported widths.

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

- The footer is derived from current-context actions and existing typed
  keybindings rather than a decorative fixed list.
- Every displayed shortcut dispatches the named action, and footer/help labels
  agree with the authoritative keymap.
- Unavailable routes are omitted from the rail or shown only with an explicit
  disabled reason where the documented context requires discoverability.
- Wide layouts preserve the documented high-value global/workspace routes;
  medium and narrow layouts hide lower-priority actions deterministically.
- Dialog, command-palette, editor, terminal-prefix, and workspace-specific
  traps retain their complete keyboard behavior.
- The fixed-width clock and shell border remain stable without crowding the
  shortcut rail.
- High-contrast, no-color, reduced-motion, and minimum-width layouts remain
  readable and panic-free.

## Verification

```bash
cargo test -p yoctui-ui next_generation_footer
cargo test -p yoctui-app keymap
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
