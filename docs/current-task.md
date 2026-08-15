# Current Task

## Task

**ID:** DAEMON-001
**Title:** Complete persistent Yoctui daemon session architecture
**Status:** IN_PROGRESS

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
were reused from the shared cache. The final completion gate exposed a
parallel-test isolation collision in `daemon_persist`; the daemon parent remains
in progress until that regression is fixed and the full gate passes.
