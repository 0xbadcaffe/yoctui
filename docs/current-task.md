# Current task

## Active task

**ID:** UI-RESP-001
**Title:** Complete responsive wide, medium, narrow, and too-small layouts

## Objective

Implement the complete responsive shell behavior specified in
`docs/ui-spec.md`: three persistent panes on wide terminals, navigator plus
workspace with a toggleable inspector on medium terminals, a visible
one-pane switcher on narrow terminals, and a safe resize message below the
minimum size.

## Required work

1. Inspect the current shell renderer, application focus/navigation model,
   input mapping, and existing breakpoint tests before changing code.
2. Define stable wide, medium, narrow, and too-small breakpoint behavior
   without duplicating screen-specific layouts.
3. Keep Navigator, Workspace, and Inspector visible together in wide mode.
4. Keep Navigator and Workspace visible in medium mode and provide a
   keyboard-accessible inspector overlay or tab.
5. Render one pane at a time in narrow mode with a visible pane switcher and
   keyboard navigation among Navigator, Workspace, and Inspector.
6. Preserve existing modal focus trapping and do not let responsive pane
   controls bypass dialogs.
7. Ensure every supported screen and relevant dialog renders without panic at
   boundary dimensions and after resize transitions.
8. Add reducer/input tests for pane selection and Ratatui `TestBackend` tests
   for semantic content at every breakpoint.
9. Update `docs/ui-spec.md` in the same commit if implementation requires a
   more precise intentional shortcut or breakpoint contract.

## Definition of done

- Wide, medium, narrow, and too-small behavior matches `docs/ui-spec.md`.
- Medium terminals can reveal and dismiss the Inspector without losing the
  underlying workspace.
- Narrow terminals visibly identify and switch the active pane.
- Resize transitions clamp or preserve pane state safely.
- No supported screen or dialog panics at tested boundary dimensions.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-ui responsive
cargo test -p yoctui-app responsive
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`FOCUS-001 — Complete the shared focus router and modal focus trapping`
