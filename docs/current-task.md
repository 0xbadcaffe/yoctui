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
were reused from the shared cache. The completion-gate regression now gives
every `daemon_persist` fixture a PID-plus-monotonic temporary identity; five
consecutive parallel integration runs pass. The next full gate exposed a timeout
while the real PTY integration awaited its Running state, so the parent remains
in progress pending diagnosis and a clean full-gate pass.
