# Current Task

## Task

**ID:** UI-LITERAL-UX-001
**Title:** Make reference focus and theme controls operational
**Status:** IN_PROGRESS

## Objective

Ensure every displayed canonical F-key invokes its named typed action, pane
focus is predictable, and the theme picker visibly previews and persists.

## Dependencies

- `UI-LITERAL-COCKPIT-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui/src/main.rs`
- `scripts/test-tui-snapshots.sh`
- `crates/yoctui-ui/tests/golden/literal-reference-160x48.cells`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- F1 through F10 map to Help, Tasks, Jobs, Terminal, Logs, Layer, Recipe,
  Image, Search, and Menu respectively.
- Tab and Shift+Tab cycle Navigator, Workspace, and Inspector predictably.
- F10 exposes Choose theme without requiring a hidden chord.
- Theme preview is immediate; Enter persists and Esc restores the prior theme.
- PTY snapshots retain the canonical rail without terminal contamination.

## Verification

```bash
cargo test -p yoctui-model theme
cargo test -p yoctui-app focus
cargo test -p yoctui-ui literal_ux
./scripts/test-tui-snapshots.sh
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
