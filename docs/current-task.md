# Current Task

## Task

**ID:** DAEMON-STATE-JOBS-001
**Title:** Move all long-lived job families into daemon state
**Status:** IN_PROGRESS

## Objective

Migrate bounded logs, errors, task/build history, background jobs,
QEMU/Wic/SDK/testing/QA/security/maintenance/utility workflow state, and PTY
session metadata behind the daemon-global boundary while reusing the existing
typed models and reducer/event infrastructure.

## Verification

```bash
cargo test -p yoctui-model daemon_state_jobs
cargo test -p yoctui-app daemon_state_jobs
```
