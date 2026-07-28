# Current task

## Active task

**ID:** WIC-UI-CLI-001
**Title:** Integrate Wic creation capability and runner in the CLI

## Objective

Connect the typed Wic creation workflow to CLI-owned capability inspection,
shell-free process execution, nonblocking polling, cancellation, and exact
terminal history without coupling it to BitBake, image scans, or QEMU.

## Required work

1. Inspect the existing CLI main loop, effect dispatch, image capability
   refresh, managed-QEMU coordinator/polling, dialog-first key routing, and Wic
   adapter APIs before editing; do not duplicate their lifecycle behavior.
2. Inspect Wic capability for each exact normalized Images inventory and active
   build context, including configured kickstart identities when authoritative
   metadata provides them; dispatch the typed loaded result without guessing
   missing Yocto paths.
3. Execute `StartWicSession` by rebuilding and independently revalidating the
   exact preview against the latest capability, then start one CLI-owned
   `WicJobRunner` beneath the active build/output directory.
4. Poll Wic runner events nonblockingly and normalize them through the existing
   app boundary into starting/running, bounded stdout/stderr, success with exact
   output inventory, nonzero failure, cancellation, rejection, and process-loss
   reducer actions.
5. Execute cancellation only for the exact active session, preserve its
   confirmed request and background-job history across navigation, and keep Wic
   ownership independent from the BitBake, image-scan, package, signature,
   Devtool, and QEMU coordinators.
6. Route Wic creation/preview/cancellation modal keys before global shortcuts so
   no pane, build, QEMU, or artifact action leaks through focus trapping.
7. Add `wic_workspace` CLI tests with fake capability/process seams for
   discovery, exact arguments, modal routing, navigation persistence, output
   scan, success, nonzero failure, graceful/forced cancellation, rejection,
   duplicate start, and unexpected runner loss. Do not claim live Wic support
   from these tests.
8. Run focused and baseline checks, then mark the child done and hand off to the
   next eligible highest-priority task.

## Definition of done

- The real CLI owns one revalidated Wic creation runner and never invokes a
  shell.
- Modal keys are handled before global input and all runner outcomes become
  typed persistent state.
- Wic execution does not block the terminal loop or interfere with other
  coordinators.
- Fake integration coverage proves exact arguments and lifecycle behavior;
  live compatibility remains unclaimed until a live Wic environment passes.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui -- wic_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
