# Current task

## Active task

**ID:** QEMU-001
**Title:** Verify the complete managed QEMU parent gate

## Objective

Verify the complete managed runqemu workflow across its model, adapter, app,
responsive UI, and CLI integration children.

## Required work

1. Inspect the completed child task evidence and dependencies before changing
   implementation.
2. Run every focused verification command for `QEMU-001`.
3. Confirm exact artifact identity and shell-free preview revalidation remain
   enforced from capability inspection through process launch.
4. Confirm shared job lifecycle, bounded output, navigation persistence,
   responsive rendering, and graceful/forced cancellation agree across layers.
5. Confirm the status and architecture documents make no live runqemu
   compatibility claim from fake executable tests.
6. Run all baseline checks.
7. If every check passes, mark `QEMU-001` done and select the next eligible
   highest-priority task. If a check fails, repair only the demonstrated
   integration defect.

## Definition of done

- The complete parent verification passes.
- Model, adapter, app, UI, and CLI boundaries remain typed and consistent.
- Exact launch arguments and terminal lifecycle outcomes are covered.
- Fake integration evidence is not presented as live compatibility.
- Baseline checks pass without weakening tests.

## Verification

```bash
cargo test -p yoctui-app qemu
cargo test -p yoctui-ui qemu
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
