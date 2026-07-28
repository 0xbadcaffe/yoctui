# Current task

## Active task

**ID:** WIC-WRITE-UI-CLI-001
**Title:** Integrate protected Wic device writing

## Objective

Close the protected Wic device-writing parent gate by verifying that the
completed adapter, model, app, responsive UI, and CLI execution paths agree on
one typed, fail-closed workflow.

## Required work

1. Inspect the completed Wic write child tasks and confirm their exact identity,
   lifecycle, dialog, rendering, and CLI ownership contracts agree.
2. Run every focused cross-layer verification command. Add only genuinely
   missing regression coverage; do not duplicate child behavior or weaken a
   check.
3. Verify discovery and write execution remain shell-free, bounded,
   generation-correlated, nonblocking, and adapter-revalidated immediately
   before spawn.
4. Verify responsive/no-color dialogs, write history/telemetry, modal routing,
   stronger cancellation acknowledgement, and every terminal outcome remain
   covered across navigation.
5. Keep the safety claim honest: fake node/process tests prove integration, not
   live removable-media compatibility. Do not perform a hardware write.
6. Run all baseline checks, reconcile roadmap/status documentation, and hand
   off to the next eligible highest-priority task.

## Definition of done

- All four focused Wic device-write suites pass together.
- All baseline checks pass.
- Adapter, model, app, UI, and CLI contracts agree without duplicate logic.
- Documentation records the completed integration without a live-hardware
  claim.

## Verification

```bash
cargo test -p yoctui-model wic_device_write
cargo test -p yoctui-app wic_device_write
cargo test -p yoctui-ui wic_device_write
cargo test -p yoctui -- wic_device_write
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
