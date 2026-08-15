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
real `core-image-minimal` run originally reached task 2459 of 4090 before the
host fell to 4 GB free (100% filesystem use), so it was stopped to avoid
exhausting the filesystem. After recovering sufficient space, a fresh run
remained healthy through task 2779 of 4090 but exposed that the former one-hour
harness deadline was too short for an uncached build on this host. The default
deadline is now four hours. Rerun the live acceptance before changing this
status. The harness also excludes inherited pyenv shim state after that state
was shown to stall BitBake host-tool execution. A subsequent run remained
healthy through task 4055 of 4090 before the disposable workspace exhausted
the filesystem during kernel compilation. The harness now enables Poky's
standard `rm_work` class to reclaim completed recipe workdirs; rerun with the
accumulated shared cache before changing this status.
