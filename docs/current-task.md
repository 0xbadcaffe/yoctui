# Current task

## Active task

**ID:** QEMU-UI-CLI-001
**Title:** Integrate QEMU capability and runner in the CLI

## Objective

Execute QEMU capability/start/cancel effects, route modal keys, and
non-blockingly poll one managed runner beside backend, BitBake, Devtool, and
other background work.

## Required work

1. Inspect the CLI's image-artifact completion, effect dispatch, dialog input
   priority, Devtool runner lifecycle, and event-loop polling before editing.
2. After each successful/partial/empty artifact inventory result, inspect
   runqemu capability against that exact normalized inventory and dispatch the
   typed capability state. Failed/cancelled inventories must not retain stale
   availability.
3. Route QEMU launch/preview/cancellation dialog keys before workspace/global
   shortcuts so modal input cannot leak.
4. Execute `StartQemuSession` by rebuilding the deterministic preview from the
   current typed capability/request, constructing `QemuCommandSpec`, and
   starting one `QemuJobRunner` in the active build directory.
5. Convert preflight/start failures into typed session failure actions with
   exact details; never leave a queued session active after failure.
6. Non-blockingly poll runner events, normalize them through
   `qemu_actions_for_runner_event`, and clear ownership only after terminal
   events while preserving model history across navigation.
7. Execute confirmed cancellation, dispatch cancellation rejection on false or
   error, and allow the runner's graceful/forced terminal event to finish the
   session.
8. Keep QEMU polling independent from BitBake, Devtool, signature, package, and
   image scanning; no blocking wait may enter keyboard/backend selection.
9. Add CLI tests named `qemu_workspace` using fake executables for capability
   refresh, exact successful launch, nonzero failure, cancellation rejection/
   forced completion, process loss, navigation persistence, and dialog key
   priority.
10. Update `docs/architecture.md` with CLI ownership and polling, then mark the
    child done and hand off to the `QEMU-UI-001` parent gate.

## Definition of done

- The end-to-end managed QEMU path is connected through typed effects/events.
- UI remains responsive and independent of other job coordinators.
- Fake integration tests pass; live runqemu compatibility is not claimed.
- Focused and baseline checks pass.

## Verification

```bash
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
