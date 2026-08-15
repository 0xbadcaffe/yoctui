# Current Task

## Task

**ID:** DAEMON-001
**Title:** Complete persistent Yoctui daemon session architecture
**Status:** DONE

## Objective

Parent gate: daemon-owned BitBake/jobs/PTYs survive client and SSH disconnect,
safely support clients/input ownership/keyboard/mouse, enforce limits/security,
state honest reboot semantics, and pass real Poky acceptance.

## Verification

```bash
./scripts/verify-completion.sh
./scripts/live-daemon-poky.sh
```

The daemon-owned BitBake build path and live validation harness are complete.
With the host namespace prerequisite enabled, the final real Poky scarthgap
`core-image-minimal` acceptance run passed fresh clone and initialization,
daemon start, detached submission, repeated reconnects, and terminal job-state
reporting. All 4567 tasks succeeded, including kernel and image creation; 3648
were reused from the shared cache. The completion-gate regression now gives
every `daemon_persist` fixture a PID-plus-monotonic temporary identity; five
consecutive parallel integration runs pass. Server IPC now separates short read
slices from bounded multi-second snapshot writes; the real PTY integration
passes ten consecutive runs. Every daemon-state fixture now has a
PID-plus-monotonic identity; all six cases, including both SSH acceptance names,
pass ten consecutive parallel runs. The next full gate reached the UI suite and
found that Configuration action guidance displaced earlier provenance at
100x25. Authoritative values, provenance, overrides, and operations now render
first, followed by compact action state and exact reasons; all 121 UI tests pass.
This is the terminal handoff because every registry task is DONE.
