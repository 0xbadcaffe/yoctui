# Current Task

## Task

**ID:** BITBAKE-CLI-CONTROL-001
**Title:** Provide shell-free BitBake CLI control fallback
**Status:** IN_PROGRESS

## Objective

Provide typed, capability-aware BitBake CLI control paths for operations where
the supported socket adapter is insufficient. Every command must use a
shell-free argv vector, expose an exact preview, bound output and runtime,
support cancellation, and report typed outcomes without depending on UI state.

## Verification

```bash
cargo test -p yoctui-bitbake cli_control
```
