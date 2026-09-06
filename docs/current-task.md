# Current Task

## Task

**ID:** WORKBENCH-INTEGRATION-001
**Title:** Integrate kernel firmware rootfs and Overview with the performance release
**Status:** IN_PROGRESS

## Objective

Integrate all four feature branches with the completed M46 performance release.
Preserve provider authority, typed effects, bounded data, scheduling, and IPC.
Reconcile Navigator identities, mouse dispatch, release version, and visual fixtures.

## Dependencies

- PERF-001, KERNEL-WORKBENCH-001, FIRMWARE-WORKBENCH-001,
  ROOTFS-SYSTEM-EXPLORER-001, OVERVIEW-001 — DONE on their source branches.

## Definition of done

- All four feature tips are ancestors of the integrated release.
- All destinations and inputs coexist and reviewed production fixtures match.
- Workspace, bridge, Clippy, roadmap, documentation, and performance gates pass.
- The full completion verifier passes before master is pushed.

## Verification

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-performance.sh
./scripts/verify-completion.sh
```
