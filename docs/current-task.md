# Current task

## Active task

**ID:** QEMU-UI-001
**Title:** Integrate managed QEMU dialogs and session view

## Objective

Integrate capability-driven runqemu launch editing, deterministic confirmation,
CLI-owned persistent execution/cancellation, and responsive attached session
inspection into the Images workspace.

## Required work

1. Inspect Images workspace rendering/input/effects, existing typed dialog
   editors, Devtool CLI runner polling, and background-job views before adding
   overlapping behavior.
2. Inspect runqemu capability after authoritative image artifacts load or
   refresh and preserve every distinct capability/disabled state.
3. Add the documented Images shortcut and footer hint for launching the exact
   selected compatible root-filesystem/Wic artifact. Unsupported selections,
   missing tool/images, failed inspection, and active sessions must show stable
   typed explanations.
4. Render a focus-trapped responsive launch editor for machine/image, optional
   kernel/rootfs, networking, display, serial, memory, and extra arguments.
   Keep identity fields read-only and all editable fields bounded.
5. Render the deterministic exact argument preview and require explicit
   confirmation. `Esc` from draft/preview must close without starting.
6. Execute `InspectQemuCapability`, `StartQemuSession`, and
   `CancelQemuSession` effects in the CLI. Own and non-blockingly poll one
   `QemuJobRunner` beside backend, keyboard, and other background work.
7. Route typed runner events through app normalization, preserve sessions
   across navigation, report start/cancellation failures, and never block
   BitBake or Devtool coordination.
8. Render the active/latest managed session in the Images Inspector with exact
   request, lifecycle/timestamps, retained stdout/stderr, truncation/drop
   counts, exit/error, and confirmed cancellation action.
9. Cover explicit capability states, editor/preview/cancel keys, success,
   nonzero failure, cancellation rejection/forced completion, process loss,
   navigation persistence, and 80x24/100x30/160x40 TestBackend layouts in tests
   named `qemu_workspace`.
10. Update `docs/ui-spec.md` for exact shortcuts/focus/responsive behavior and
    `docs/architecture.md` for CLI ownership, then mark the child done and hand
    off to the `QEMU-001` parent gate.

## Definition of done

- A compatible selected image launches only after validated preview and
  confirmation through the managed adapter.
- Session lifecycle/output/cancellation remain responsive and visible.
- Reducer, app, CLI, and TestBackend focused tests plus baseline checks pass.
- No live runqemu compatibility is claimed from fake tests alone.

## Verification

```bash
cargo test -p yoctui-model qemu_workspace
cargo test -p yoctui-app qemu_workspace
cargo test -p yoctui-ui qemu_workspace
cargo test -p yoctui -- qemu_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
