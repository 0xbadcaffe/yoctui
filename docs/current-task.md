# Current task

## Active task

**ID:** QEMU-001
**Title:** Complete the managed runqemu workflow

## Objective

Add a typed managed runqemu workflow with explicit launch configuration,
persistent process state, console/log inspection, and safe cancellation.

## Required work

1. Inspect all existing shell/process/background-job, Images, dialog, and
   terminal-lifecycle behavior before writing code.
2. Reconcile this broad parent into atomic child tasks if it cannot be
   completed as one coherent verified commit.
3. Detect runqemu capability and compatible built image artifacts using only
   authoritative workspace/adapter state.
4. Add typed launch configuration and preview/confirmation for machine, image,
   networking, serial/display, memory, and extra validated options.
5. Execute runqemu without a shell as a persistent cancellable background
   process with bounded stdout/stderr and distinct start/failure/exit/loss
   states.
6. Provide console/log inspection without suspending the persistent workbench;
   terminal ownership must remain explicit and recoverable.
7. Require explicit confirmation for launch and cancellation; prevent
   duplicate active sessions.
8. Render responsive dialogs/workspace state and stable disabled explanations
   in every theme/no-color mode.
9. Add model, app, adapter, CLI integration, and Ratatui TestBackend tests plus
   live smoke validation when a compatible built image is available.
10. Update `docs/ui-spec.md`, `docs/architecture.md`, registry, and status in
    the same coherent commits.

## Definition of done

- A configured runqemu session can be launched, inspected, and cancelled
  through typed persistent state.
- Unsupported/missing tools and images remain explicit and non-fatal.
- Focused and baseline checks pass; live claims require live evidence.

## Verification

```bash
cargo test -p yoctui-model qemu
cargo test -p yoctui-app qemu
cargo test -p yoctui-ui qemu
cargo test -p yoctui -- qemu
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
