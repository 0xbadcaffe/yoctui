# Current Task

## Task

**ID:** WORKBENCH-INTEGRATION-001
**Title:** Integrate kernel firmware rootfs and Overview with the performance release
**Status:** DONE

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

## Verification progress

The final source checkpoint is `78eb9bf` (v0.1.57); all four feature tips
are ancestors. Workspace tests, strict Clippy, bridge checks, reviewed production
goldens/rasters, coverage, fuzz/stress, sanitizers, and a fresh CPU flamegraph
passed. Fresh v0.1.57 idle evidence measures 0.1664% combined daemon/client CPU
against one logical CPU.

Rendering verification exposed a repeatable large-editor regression:
10.15–11.86 ms/frame against the unchanged 10 ms threshold. A 2,144-sample
CPU profile attributed 43.38% of self time to SHA-256. Exact text comparison
now replaces boolean dirty/diff hashing; save/conflict revision hashes remain
unchanged. Three quiet release repeats measured 8.663, 8.056, and 7.224 ms/frame;
the full five-scenario matrix passed with the editor at 7.195 ms/frame.
Both baseline and result are retained under `artifacts/performance/editor/`,
including an unaccepted run that overlapped profiling.

Recorded real-Poky, saturated input/IPC, and 30-minute endurance evidence remains
bound to the preceding `7dab483` (v0.1.56) checkpoint, before the editor-only
optimization. The evidence validators check the unchanged relevant sources;
these records are not presented as fresh v0.1.57 live-build measurements.
The real build measured 0.9858% combined CPU and 8.009 ms input p95, with
cancellation and reconnect passing. Endurance preserved bounded memory,
critical events, ordering, and continuity.

The independent full completion gate remains the mandatory release boundary:
run it on the clean final merge candidate before advancing or pushing master.
All task-specific gates, including the aggregate performance verifier and
expanded-workbench rendering matrix, passed. This is the terminal task handoff;
there are no remaining incomplete registry tasks.

## Verification

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-performance.sh
./scripts/test-workbench-ux-performance.sh
./scripts/verify-completion.sh
```
