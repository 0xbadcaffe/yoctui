# Current task

## Active task

**ID:** FOCUS-001
**Title:** Complete the shared focus router and modal focus trapping

## Objective

Make Navigator, Workspace, Inspector, Dialog, and CommandPalette use one
predictable focus router across every workspace and transient interaction,
including restoration to the exact prior pane after a modal closes.

## Required work

1. Inspect every focus assignment in the reducer, application input mapping,
   CLI dispatch loop, and renderer before changing state.
2. Centralize entry to and exit from Dialog and CommandPalette focus so the
   prior non-modal focus target is recorded and restored.
3. Ensure Tab and Shift+Tab cycle Navigator, Workspace, and Inspector in both
   directions with wraparound.
4. Ensure arrow keys and activation keys affect only the focused region.
5. Make Esc close the innermost transient mode or return focus outward without
   replacing the active workspace.
6. Ensure every dialog and the command palette trap focus until closed,
   including when other workspace state such as the layer browser is active.
7. Preserve responsive pane focus and visible focus styling after dialogs
   close and across workspace changes.
8. Add reducer tests for focus entry/restoration and invalid transitions,
   application/CLI input-routing tests, and Ratatui tests for visible focus and
   modal trapping.
9. Update `docs/ui-spec.md` in the same commit if the shared routing contract
   needs clarification.

## Definition of done

- Exactly one focus target is active.
- Pane cycling is bidirectional, bounded by wraparound, and modal-safe.
- Dialog and command-palette close paths restore the exact prior pane.
- Modal input cannot reach navigator or workspace actions.
- Esc follows the specified transient-mode and outward-focus behavior.
- Focus is visibly distinguishable at all responsive breakpoints.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model focus
cargo test -p yoctui-app focus
cargo test -p yoctui-ui dialog
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DIALOG-001 — Implement the unified typed dialog stack`
