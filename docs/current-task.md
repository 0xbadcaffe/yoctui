# Current Task

## Task

**ID:** FINAL-GATE-PERF-001
**Title:** Rerun the terminal gate with perf sampling enabled
**Status:** BLOCKED

## Objective

Complete the final clean-checkout gate with real Linux `perf` sampling and a
fresh deterministic Yoctui flamegraph.

## Dependencies

- `CRATESIO-COVERAGE-001` — DONE
- `UI-STARTUP-DIAG-001` — DONE

## Relevant files

- `scripts/flamegraph.sh`
- `scripts/verify-completion.sh`
- `artifacts/flamegraph/yoctui.svg`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- `perf record -- true` succeeds with real sampling permission.
- `./scripts/flamegraph.sh` records a fresh deterministic flamegraph from the
  real Yoctui release workload.
- `./scripts/verify-completion.sh` passes.
- The task is marked DONE only after those commands pass.

## Blocker

On 2026-08-19, `perf record --no-buildid-mmap -e dummy:u -- true` fails because
this host has `kernel.perf_event_paranoid=4` and the current process has no
`CAP_PERFMON`, `CAP_SYS_PTRACE`, or `CAP_SYS_ADMIN`. Changing that host security
policy requires operator authority; product tests cannot substitute for real
sampling.

## Verification

```bash
perf record -- true
./scripts/flamegraph.sh
./scripts/verify-completion.sh
```
