# Current task

## Active task

**ID:** IMAGES-001
**Title:** Verify the complete Images artifact workspace

## Objective

Verify that the completed model, adapter, and UI children jointly satisfy the
Images artifact workspace contract without overstating live compatibility.

## Required work

1. Re-read the three child task notes and inspect their implementation/tests.
2. Run every parent verification command plus the full baseline.
3. Confirm existing image recipe selection and build confirmation tests remain
   green alongside artifact model, adapter, background, and TestBackend tests.
4. Confirm the UI and architecture specifications match intentional behavior.
5. Do not claim live deployed-artifact compatibility unless an initialized
   build with authoritative `DEPLOY_DIR_IMAGE` is actually exercised.
6. Mark the parent `DONE` only when every check passes, update status, select
   `QEMU-001`, and commit the governance handoff.

## Definition of done

- Every child is `DONE` with passing evidence.
- The parent and baseline verification commands pass.
- Documentation makes no unsupported live compatibility claim.

## Verification

```bash
cargo test -p yoctui-model images_workspace
cargo test -p yoctui-ui images_workspace
cargo test -p yoctui-app image_action
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
