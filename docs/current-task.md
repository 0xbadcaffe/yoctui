# Current task

## Active task

**ID:** QEMU-UI-RENDER-001
**Title:** Render responsive QEMU launch and session UI

## Objective

Render capability/availability, the typed launch editor and exact preview,
latest managed session lifecycle/output/error details, cancellation
confirmation, and Images footer hints at every supported breakpoint.

## Required work

1. Inspect Images responsive Workspace/Inspector rendering, semantic theme
   helpers, existing dialog layouts, and background-job presentation before
   adding widgets.
2. Show the exact runqemu capability state and stable launch disabled reason in
   the Images Inspector without inferring filesystem or process state.
3. Render the launch dialog's selected/read-only/editing rows, typed choices,
   bounds, inline validation, and exact footer controls.
4. Render deterministic preview arguments without shell quoting claims and
   require explicit confirmation.
5. Render cancellation confirmation with exact session/image identity.
6. Render the latest session request, shared-job lifecycle and timestamps,
   stream-tagged bounded output, truncation/drop counts, exit code, and typed
   error/result details.
7. Add `Q` launch and `x` cancellation footer hints while preserving existing
   Images actions and narrow behavior.
8. Ensure 80x24, 100x30, and 160x40 plus the global too-small boundary never
   panic or lose the active dialog/session meaning.
9. Add Ratatui `TestBackend` tests named `qemu_workspace` for capability
   states, draft/validation/preview/cancellation dialogs, lifecycle/output,
   terminal outcomes, and all supported dimensions.
10. Keep rendering pure; do not inspect capabilities or own/poll processes.
    Mark the child done and hand off to `QEMU-UI-CLI-001`.

## Definition of done

- Typed QEMU state is complete and understandable in every supported layout.
- Dialog selection/editing/validation and session terminal states are visible.
- Existing Images behavior remains intact.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-ui qemu_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
