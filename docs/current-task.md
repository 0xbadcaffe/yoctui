# Current Task

## Task

**ID:** SHELL-TEST-001
**Title:** Add PTY end-to-end tests for the embedded shell
**Status:** NOT_STARTED

## Objective

Drive a real shell through the outer Yoctui PTY and nested child PTY, verifying
interactive input, resize, Unicode, scrollback, escape, cleanup, and terminal
restoration.

## Verification

```bash
cargo test -p yoctui-e2e embedded_shell
./scripts/test-embedded-shell.sh
```

## Definition of done

- Outer and nested PTY evidence covers shell lifecycle, focus restoration,
  cleanup, and terminal restoration.

## Next task

After completion, select `SHELL-DOC-001`.
