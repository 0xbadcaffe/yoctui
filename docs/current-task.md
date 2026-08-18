# Current Task

## Task

**ID:** UI-FOCUS-ROUTING-001
**Title:** Preserve global and workspace keys under pane focus
**Status:** IN_PROGRESS

## Objective

Make the CLI consume only keys that actually map to pane focus, allowing every
unmatched key to continue to the active workspace and global shortcut routes.

## Dependencies

- `UI-STARTUP-STDERR-001` — DONE

## Relevant files

- `crates/yoctui-cli/src/main.rs`
- `crates/yoctui-app/src/lib.rs`
- `docs/ui-spec.md`

## Definition of done

- `Ctrl+P`, help, build, and workspace keys are not swallowed by pane focus.
- Navigator, Workspace, and Inspector retain their mapped focus behavior.
- A CLI routing regression test covers matched and unmatched focus input.
- Focused tests, formatting, Clippy, docs, and roadmap checks pass.

## Verification

```bash
cargo test -p yoctui focus_routing
cargo test -p yoctui-app focus
cargo fmt --all --check
cargo clippy -p yoctui --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
