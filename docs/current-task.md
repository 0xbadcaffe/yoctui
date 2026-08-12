# Current Task

## Task

**ID:** HOST-REBOOT-001
**Title:** Define and implement host reboot behavior
**Status:** IN_PROGRESS

## Objective

Define and verify the stronger host-reboot boundary. Support daemon auto-start
through the documented user service, list persisted sessions, mark reboot-killed
work `Lost` or stopped, and expose only explicit typed relaunch intent. Never
claim arbitrary child processes or PTYs survived a changed boot identity.

## Verification

```bash
cargo test -p yoctui reboot_recovery
./scripts/check-docs.sh
```
