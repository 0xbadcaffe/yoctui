# Current Task

## Task

**ID:** FINAL-GATE-PERF-001
**Title:** Rerun the terminal gate with perf sampling enabled
**Status:** IN_PROGRESS

## Objective

Run the complete repository completion gate while real Linux perf sampling is
temporarily available, record the terminal result, and restore the host's
original perf security policy afterward.

## Dependencies

- `CRATESIO-COVERAGE-001` — DONE
- `UI-STARTUP-DIAG-001` — DONE
- `PERF-FLAMEGRAPH-QUALITY-001` — DONE

## Relevant files

- `scripts/verify-completion.sh`
- `scripts/flamegraph.sh`
- `artifacts/flamegraph/yoctui.svg`
- `artifacts/flamegraph/summary.txt`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Real `perf record` sampling succeeds.
- `./scripts/flamegraph.sh` passes its workload and symbol-quality gates.
- `./scripts/verify-completion.sh` passes without skipped required checks.
- The registry, implementation status, and terminal current-task handoff record
  final completion.
- The original `kernel.perf_event_paranoid=4` policy is restored or an exact
  operator command is reported if credentialed restoration remains external.

## Verification

```bash
perf record -- true
./scripts/flamegraph.sh
./scripts/verify-completion.sh
```
