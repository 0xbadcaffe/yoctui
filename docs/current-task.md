# Current task

## Active task

**ID:** QEMU-UI-001
**Title:** Verify the complete QEMU UI parent gate

## Objective

Verify that the completed QEMU dialog state, responsive rendering, and
CLI-owned capability/runner integration form one coherent typed workspace.

## Required work

1. Read the completed QEMU model, app input, UI rendering, and CLI integration
   tests before changing any implementation.
2. Run every focused verification command for the parent task.
3. Confirm the launch, confirmation, cancellation, active-session, and terminal
   outcome paths remain typed and responsive at supported terminal sizes.
4. Confirm fake executable coverage is described honestly and no live runqemu
   compatibility claim was introduced.
5. Run all baseline checks.
6. If every check passes, mark `QEMU-UI-001` done and hand off to the
   `QEMU-001` managed-workflow parent gate. If a check fails, keep this task in
   progress and repair only the demonstrated integration defect.

## Definition of done

- All four focused model/app/UI/CLI checks pass together.
- Modal focus, responsive rendering, and session history agree with
  `docs/ui-spec.md`.
- Fake integration evidence is not presented as live compatibility.
- Baseline checks pass without weakening tests.

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
