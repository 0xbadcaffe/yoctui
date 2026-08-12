# Current Task

## Task

**ID:** PROFILE-LOAD-001
**Title:** Load and generate project profiles safely
**Status:** IN_PROGRESS

## Objective

Load optional profiles without vendor or layer changes, generate safe profiles
only through explicit typed actions, and keep personal settings user-local.

## Verification

```bash
cargo test -p yoctui project_profile
cargo test -p yoctui-app project_profile
```
