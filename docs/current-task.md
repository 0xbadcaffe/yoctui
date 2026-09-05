# Current Task

## Task

**ID:** PERF-REAL-POKY-001
**Title:** Validate responsiveness during a real saturated Poky build
**Status:** IN_PROGRESS

## Objective

Capture sustained supported-Poky task execution with the optimized daemon and
client while the build uses the available host CPUs.

## Dependencies

- PERF-CPU-GATE-001 — DONE
- PERF-RESPONSIVENESS-GATE-001 — DONE
- PERF-IPC-GATE-001 — DONE
- PERF-MEMORY-GATE-001 — DONE

## Definition of done

- Evidence names the supported Poky revision, build directory, target, machine,
  distro, exact Yoctui binary, host, and measurement window.
- Sustained real task execution captures daemon/client/BitBake CPU and memory,
  render/event rates, input and IPC latency, and pressure counters.
- Daemon/client and BitBake backend continuity hold under real host pressure;
  input remains responsive and cancellation evidence is explicit.
- The artifact is labeled real-Poky evidence and cannot be substituted by a
  fixture-only run.

## Verification

```bash
./scripts/verify-performance.sh --real-poky-evidence
./scripts/verify-roadmap.sh
```

The bounded-memory gate is complete in v0.1.46.
