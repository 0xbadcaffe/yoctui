# Current task

## Active task

**ID:** DIALOG-001
**Title:** Implement the unified typed dialog stack

## Objective

Replace the independent boolean/optional modal fields with one typed dialog
state and explicit transitions, while preserving all existing build, image,
recipe, Devtool, BBMASK, editor, quit, and completion behavior.

## Required work

1. Inventory every modal field, renderer branch, reducer action, input branch,
   confirmation effect, and existing test before changing representation.
2. Define a typed `Dialog` model whose variants carry the state required by
   each modal workflow; use a stack only where asynchronous or nested dialogs
   genuinely require more than one retained modal.
3. Make opening, transitioning, confirming, cancelling, and dismissing dialogs
   explicit reducer transitions with invalid transitions leaving state intact.
4. Preserve the focus router's exact return target until the final dialog
   closes.
5. Route input through the active typed dialog before navigator, workspace, or
   inspector input.
6. Render from the typed dialog state without testing a long precedence chain
   of unrelated App fields.
7. Keep destructive previews and confirmations unchanged, including exact
   commands and affected targets.
8. Ensure backend events continue to update the persistent shell while a
   dialog is open; asynchronous completion dialogs must not lose an active
   user dialog.
9. Add reducer transition/failure tests, app/CLI input-routing tests, and
   Ratatui tests for every dialog family and narrow terminals.
10. Update `docs/ui-spec.md` and `docs/architecture.md` if the explicit dialog
    contract or component boundary needs clarification.

## Definition of done

- One typed source of truth determines the active dialog and retained stack.
- Every existing modal workflow renders and dispatches through that state.
- Invalid confirm/cancel/input actions do not mutate unrelated state.
- Destructive operations retain preview and explicit confirmation.
- Focus trapping/restoration remains correct through nested transitions.
- Backend completion arriving under another dialog is retained and shown next.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model dialog
cargo test -p yoctui-app dialog
cargo test -p yoctui-ui dialog
cargo test -p yoctui -- dialog
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`THEME-001 — Complete built-in semantic themes`
