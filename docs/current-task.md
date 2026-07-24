# Current task

## Active task

**ID:** THEME-001
**Title:** Complete built-in semantic themes

## Objective

Complete the five built-in themes so every shell region, status, severity,
selection, focus, progress, dialog, and disabled state is rendered through
named semantic roles rather than scattered direct colors.

## Required work

1. Inventory direct `Color` and `Style` construction in `yoctui-ui` before
   changing the representation.
2. Define a semantic theme palette for all five built-in themes: Dark, Light,
   Matrix Green, High Contrast, and Monochrome.
3. Cover foreground/background, border, focused border, selection, disabled,
   info, success, warning, error, progress, text accent, and code syntax roles.
4. Route shell, workspace, inspector, footer, dialogs, notifications, gauges,
   tables, logs, and source previews through those roles.
5. Preserve `--no-color` behavior using terminal attributes and readable
   monochrome contrast without relying on color.
6. Ensure focus and selection remain visibly distinct in every theme and on
   narrow layouts.
7. Add deterministic TestBackend coverage for semantic roles, all theme
   variants, no-color mode, dialogs, progress, and severity rendering.
8. Update `docs/ui-spec.md` with the final semantic role contract.

## Definition of done

- UI code no longer uses scattered direct colors for product state.
- Every built-in theme supplies every semantic role.
- Focus, selection, severity, progress, disabled state, and dialogs are
  distinguishable in all themes.
- `--no-color` and Monochrome remain readable and deterministic.
- Existing layouts and behavior remain unchanged.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-ui theme
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`SETTINGS-001 — Implement interactive settings editing and persistence`
