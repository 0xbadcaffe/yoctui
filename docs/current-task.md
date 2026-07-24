# Current task

## Active task

**ID:** SETTINGS-001
**Title:** Implement interactive settings editing and persistence

## Objective

Turn the read-only Settings workspace into a typed interactive editor whose
changes apply immediately where appropriate and persist through the existing
configuration/session boundary.

## Required work

1. Inventory existing configuration precedence, `FileConfig`, session state,
   Settings rendering, and theme/animation/reduced-motion actions.
2. Define typed settings rows and reducer actions for selection and editing.
3. Support theme, animation speed, reduced motion, color enablement, log wrap,
   and log follow without an unstructured text editor.
4. Apply preview-safe visual settings immediately and preserve focus/selection.
5. Persist accepted values through the CLI-owned configuration or session
   boundary without overwriting unrelated user configuration.
6. Surface write failures as typed notices while retaining the in-memory value
   and a retryable dirty state.
7. Keep CLI/config/environment precedence explicit and documented.
8. Add model reducer tests, app input mapping tests, CLI persistence/failure
   tests, and Settings TestBackend coverage including narrow terminals.
9. Update `docs/ui-spec.md` and `docs/architecture.md` for the persistence
   contract.

## Definition of done

- Settings are selected and changed through typed actions.
- Supported visual/log preferences update predictably without leaving the TUI.
- Accepted settings survive restart through the documented precedence model.
- Persistence failures are visible and do not silently discard dirty state.
- Narrow Settings rendering is safe and exposes the active value and controls.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model settings
cargo test -p yoctui-app settings
cargo test -p yoctui-ui settings
cargo test -p yoctui -- settings
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`ANIM-001 — Complete indeterminate task animation and reduced motion`
