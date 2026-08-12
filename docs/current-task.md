# Current Task

## Task

**ID:** PROFILE-UI-001
**Title:** Render typed project profile workflows
**Status:** IN_PROGRESS

## Objective

Expose project-profile favorites, typed presets, and workflows with explicit
absent, invalid, stale, ambiguous, and unavailable states while preserving the
normal Yoctui confirmation policies.

## Verification

```bash
cargo test -p yoctui-ui project_profile
```
