# Current Task

## Task

**ID:** SHELL-INTEGRATION-001
**Title:** Integrate shell sessions with Yocto context and utilities
**Status:** NOT_STARTED

## Objective

Open shells at build/source/layer/recipe/SDK locations and preserve initialized
Yocto variables without silently re-sourcing a running shell.

## Verification

```bash
cargo test -p yoctui -- embedded_shell_integration
```

## Definition of done

- Shell sessions retain exact Yocto context and expose stale-environment state
  with controlled refresh/restart.

## Next task

After completion, select `SHELL-TEST-001`.
