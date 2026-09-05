# Current Task

## Task

**ID:** PERF-BASELINE-001
**Title:** Capture the pre-optimization performance baseline
**Status:** IN_PROGRESS

## Objective

Measure the current unoptimized daemon and attached-client runtime under every
contract baseline scenario and retain reproducible machine-readable artifacts
before changing event loops, rendering, telemetry, IPC, or reducers.

## Dependencies

- PERF-SPEC-001 — DONE

## Definition of done

- Daemon, client, and combined CPU use the contract's `/proc` and monotonic
  accounting with warmup and a 60-second sample window.
- Context switches, wakeups when available, render and IPC frequencies,
  BitBake event and telemetry rates, memory, and thread counts are recorded.
- Idle daemon, idle attached client, active build, event flood, PTY idle and
  active, and two-client scenarios are identified separately.
- Artifact metadata includes the exact binary/revision, host, terminal,
  command, scenario, windows, raw samples, and checksums.
- No optimization lands before this evidence is recorded.

## Verification

```bash
./scripts/verify-performance.sh --baseline
./scripts/verify-roadmap.sh
```

The normative contract is complete. Baseline capture is the only active M46
task before optimization.
