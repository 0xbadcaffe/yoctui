# Current Task

## Task

**ID:** DAEMON-001
**Title:** Complete persistent Yoctui daemon session architecture
**Status:** BLOCKED

## Objective

Parent gate: daemon-owned BitBake/jobs/PTYs survive client and SSH disconnect,
safely support clients/input ownership/keyboard/mouse, enforce limits/security,
state honest reboot semantics, and pass real Poky acceptance.

## Verification

```bash
./scripts/verify-completion.sh
./scripts/live-daemon-poky.sh
```

The daemon-owned BitBake build path and live validation harness are implemented.
The AppArmor user-namespace restriction was temporarily disabled and the live
harness passed its namespace preflight, fresh clone, environment initialization,
daemon start, detached build submission, and repeated reconnect checks. The
real `core-image-minimal` run reached task 2459 of 4090 before the host fell to
4 GB free (100% filesystem use), so it was stopped to avoid exhausting the
filesystem. Free sufficient build space, then rerun the live acceptance before
changing this status. The harness now also excludes inherited pyenv shim state
after that state was shown to stall BitBake host-tool execution.
