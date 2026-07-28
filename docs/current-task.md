# Current task

## Active task

**ID:** QEMU-UI-MODEL-001
**Title:** Add QEMU launch dialog and workspace input state

## Objective

Add pure bounded launch-field selection/editing, stable capability-driven
availability reasons, modal preview/cancel transitions, and typed Images
workspace/session input mapping without rendering or starting processes.

## Required work

1. Inspect existing typed dialog editors, modal focus transitions, Images input
   mapping, and QEMU launch/session types before changing state.
2. Add typed launch-field selection and edit mode for read-only identity plus
   kernel, rootfs, networking, display, serial, memory, and extra arguments.
3. Bound every text-edit action and cycle only typed choice fields. Invalid
   edits remain in the draft with a visible validation reason at preview time.
4. Add stable typed launch availability/disabled reasons for capability state,
   selection kind/identity, and duplicate active session.
5. Map the documented Images launch shortcut, launch-dialog navigation/edit/
   preview/cancel keys, preview confirmation/cancel keys, and session
   cancellation confirmation keys to typed actions.
6. Ensure all QEMU dialogs trap focus and `Esc` closes draft/preview/
   cancellation without side effects.
7. Add focused reducer and app tests named `qemu_workspace`.
8. Update `docs/ui-spec.md` with exact shortcuts and field focus/edit behavior,
   then mark the child done and hand off to `QEMU-UI-RENDER-001`.

## Definition of done

- QEMU launch input behavior is pure, bounded, focus-safe, and fully typed.
- Stable disabled reasons and all dialog transitions are tested.
- No rendering, process inspection, or process execution is added.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model qemu_workspace
cargo test -p yoctui-app qemu_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
