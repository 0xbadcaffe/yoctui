# Current task

## Active task

**ID:** QEMU-MODEL-001
**Title:** Add typed runqemu launch and session state

## Objective

Add pure typed capability, validated launch configuration, preview,
confirmation, persistent session lifecycle, bounded output, and cancellation
intent for runqemu without starting processes.

## Required work

1. Inspect existing background job, dialog/focus, image artifact, Devtool
   process lifecycle, and app-event patterns before adding overlapping state.
2. Define typed runqemu capability with available, missing-tool,
   missing-compatible-image, and failed inspection states.
3. Define an exact launch request containing machine, authoritative image
   artifact/path identity, optional kernel/rootfs, networking mode, display
   mode, serial mode, bounded memory, and validated extra argument tokens.
4. Reject empty/control/option-injection ambiguity, relative paths, identity
   mismatch, incompatible artifacts, unsafe memory values, and unsupported
   combinations before preview.
5. Add typed editable draft state, deterministic launch preview, explicit
   confirmation, and modal-safe reducer transitions.
6. Add a persistent session state with stable ID, request, queued/starting/
   running/cancelling/succeeded/failed/cancelled/lost status, timestamps,
   bounded typed stdout/stderr output, exit/error details, and cancellation
   capability.
7. Prevent duplicate active sessions and require confirmed cancellation intent.
8. Add typed actions/effects plus app event normalization. Reducers must not
   inspect executables, spawn processes, parse raw output, or own terminals.
9. Add focused tests named `qemu_model` for validation, preview, dialog focus,
   lifecycle transitions, output bounds, duplicates, cancellation, failures,
   and invalid/stale events.
10. Update `docs/architecture.md`, then mark the child done and hand off to
    `QEMU-ADAPTER-001`.

## Definition of done

- Pure state fully represents capability, launch intent, confirmation, session
  lifecycle, bounded output, and cancellation.
- Invalid or duplicate operations are reducer-inert with explicit reasons.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-model qemu_model
cargo test -p yoctui-app qemu_model
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
