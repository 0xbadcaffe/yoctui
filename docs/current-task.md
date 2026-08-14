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

The daemon-owned BitBake build path and live validation harness are implemented.
The remaining gate is blocked by the current host's AppArmor user-namespace
policy; the harness fails closed rather than claiming unsupported Poky support.
