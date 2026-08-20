# Current Task

## Task

**ID:** INPUT-TEST-002
**Title:** Test Tab and Shift-Tab flow
**Status:** IN_PROGRESS

## Objective

Exercise the complete forward and backward focus sequence through the persistent
shell, modal overlays, command palette, terminal-session view, and narrow pane
switcher without allowing input to escape the active focus owner.

## Dependencies

- `INPUT-TEST-001` — DONE
- `RESPONSIVE-UI-001` — DONE

## Relevant files

- `crates/yoctui-app/`
- `crates/yoctui-model/`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Tab and Shift+Tab traverse Navigator, Workspace, and Inspector in exact
  forward/reverse order at wide, medium, and narrow sizes.
- Dialogs and command palette trap focus and restore the prior pane when closed.
- Terminal-session focus and prefix handling do not leak into shell pane focus.
- Narrow pane switcher follows the same model focus and retains selections.

## Verification

```bash
cargo test -p yoctui-e2e next_generation_focus_flow
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
