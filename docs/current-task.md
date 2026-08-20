# Current Task

## Task

**ID:** PERF-UI-001
**Title:** Profile rendering
**Status:** IN_PROGRESS

## Objective

Measure rendering cost for idle, active-build, large recipe/layer, log-heavy,
and bounded-telemetry workloads. Produce a fresh validated flamegraph with no
null/unresolved frames, identify any real CPU hot path, and record explicit
thresholds and provenance before adding caching.

## Dependencies

- `A11Y-UI-001` — DONE
- `METRICS-UI-006` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-ui/benches/workbench_profile.rs`
- `scripts/test-next-generation-ui-performance.sh`
- `scripts/flamegraph.sh`
- `scripts/validate-flamegraph.py`
- `artifacts/flamegraph/`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- A deterministic benchmark measures all five required workload classes with
  bounded inputs and documented frame-count/time thresholds.
- Idle and active-build frame cost, large recipe/layer collections, log-heavy
  retention, and telemetry history are reported separately.
- The fresh flamegraph contains real samples, no null/unresolved frames, no
  lost samples beyond policy, and passes the independent validator.
- Exclusive/inclusive hot paths are documented from the fresh capture; a real
  actionable rendering hot path is handed to `PERF-UI-002`, while expected
  Ratatui/Unicode buffer work is not mislabeled as a defect.
- Profiling artifacts name the workload, dimensions, sample count, checksum,
  toolchain, and capture date without depending on a live daemon or Poky tree.

## Verification

```bash
./scripts/test-next-generation-ui-performance.sh
./scripts/flamegraph.sh
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
