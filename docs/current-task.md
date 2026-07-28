# Current task

## Active task

**ID:** WIC-001
**Title:** Add the Wic image workflow

## Objective

Provide a protected Wic workflow covering image creation, kickstart preview,
output inspection, and explicit device-write confirmation.

## Required work

1. Inspect the existing Images, background-job, dialog, adapter, and CLI
   implementation before writing code.
2. Reconcile the requested creation, kickstart preview, output inspection, and
   protected device-write behavior with `docs/ui-spec.md` and
   `docs/architecture.md`.
3. If the task cannot fit one coherent commit, split it into dependency-ordered
   child tasks in the registry and status documents, select the first child,
   and commit that split before implementation.
4. Implement only the resulting atomic active task with applicable unit,
   reducer, fake-process/integration, and TestBackend tests.
5. Run the focused and baseline checks before marking any task done.

## Definition of done

- Wic actions use typed model/effect boundaries.
- Kickstart and output data are bounded and safely previewed.
- External commands use validated shell-free arguments.
- Device writes show the exact target and require explicit confirmation.
- Responsive rendering and terminal outcomes are tested.

## Verification

```bash
cargo test -p yoctui-app wic
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
