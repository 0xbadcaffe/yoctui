# Current Task

## Task

**ID:** FINAL-GATE-PERF-001
**Title:** Rerun the terminal gate with perf sampling enabled
**Status:** BLOCKED

## Objective

Complete the final host-level verification with real perf sampling enabled.
All product, literal-workbench, and preceding terminal-gate stages pass; only
the Flamegraph sampling policy blocks repository completion.

## Dependencies

- `CRATESIO-COVERAGE-001` — DONE
- `UI-STARTUP-DIAG-001` — DONE

## Relevant files

- `/proc/sys/kernel/perf_event_paranoid`
- `scripts/flamegraph.sh`
- `scripts/verify-completion.sh`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- `perf record -- true` succeeds for the current user.
- A fresh real headless-workload Flamegraph is generated.
- The terminal completion gate passes without skipped required stages.

## Blocker evidence

- Current host value: `kernel.perf_event_paranoid=4`.
- `scripts/flamegraph.sh` reports that perf sampling is unavailable.
- The operator must temporarily grant `CAP_PERFMON` or lower the kernel policy.
- After changing policy, rerun the verification commands below.

## Verification

```bash
perf record -- true
./scripts/flamegraph.sh
./scripts/verify-completion.sh
```
