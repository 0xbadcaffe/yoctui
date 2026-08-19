# Current Task

## Task

**ID:** PERF-FLAMEGRAPH-QUALITY-001
**Title:** Produce a meaningful fully symbolized Yoctui flamegraph
**Status:** IN_PROGRESS

## Objective

Replace the stale backend-dependent profiling command with a deterministic
production workbench workload, capture a fresh real Linux `perf` flamegraph,
and reject unresolved or meaningless profiling evidence.

## Dependencies

- `CRATESIO-COVERAGE-001` — DONE
- `UI-STARTUP-DIAG-001` — DONE

## Relevant files

- `scripts/flamegraph.sh`
- `scripts/test-flamegraph.sh`
- `crates/yoctui-cli/benches/workbench_profile.rs`
- `crates/yoctui-cli/Cargo.toml`
- `scripts/verify-completion.sh`
- `artifacts/flamegraph/yoctui.svg`
- `artifacts/flamegraph/summary.txt`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The profiling target exercises the production reducer and Ratatui renderer
  deterministically without requiring a daemon or initialized Yocto checkout.
- `./scripts/flamegraph.sh` records a nontrivial fresh user-space flamegraph
  with resolved Yoctui application stacks and a machine-readable summary.
- The validator rejects stale/trivial output, failed workload execution,
  unresolved/null application frames, and missing dominant-symbol evidence.
- Dominant stacks are reviewed and any genuine avoidable Yoctui CPU hot path is
  fixed and recaptured before completion.

## Verification

```bash
./scripts/test-flamegraph.sh
./scripts/flamegraph.sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
