# Current task

## Active task

**ID:** WIC-UI-MODEL-001
**Title:** Add Wic workspace dialog and input state

## Objective

Add pure bounded creation dialog state, typed field editing and choices,
preview/confirmation transitions, output selection, cancellation confirmation,
disabled reasons, and app key mapping.

## Required work

1. Inspect QEMU launch dialog state, Images workspace selection, focus
   restoration, and modal input mapping before writing code.
2. Add Wic creation draft dialog rows for read-only machine, typed image and
   kickstart selection, output-directory editing, bmap choice, and compression
   choice with deterministic bounded navigation.
3. Add `W` entry from Images only when capability, active image, kickstart, and
   no active Wic operation permit it. Preserve exact disabled reasons.
4. Add `p` preview validation, exact preview confirmation, Esc close behavior,
   focus trapping/restoration, and stale capability rejection.
5. Add generated-output selection/open actions plus Wic creation cancellation
   confirmation. Keep lower-case `w` associated-artifact opening unchanged.
6. Map every dialog/workspace key mechanically in `yoctui-app`; dialogs must
   consume keys before pane/global shortcuts.
7. Add `wic_workspace` reducer/app tests for bounds, choices, validation,
   modal focus, stale/disabled paths, output selection, cancellation, and
   lower-/upper-case shortcut distinction.
8. Run focused and baseline checks, then mark the child done and hand off to
   `WIC-UI-RENDER-001`.

## Definition of done

- Dialog state is bounded, typed, modal, and capability-correlated.
- Preview/start effects cannot be emitted from stale or invalid state.
- Output selection and cancellation remain identity-stable.
- App input mapping does not parse state or leak modal keys.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model wic_workspace
cargo test -p yoctui-app wic_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
