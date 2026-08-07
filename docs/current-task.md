# Current Task

## Task

**ID:** SHELL-PTY-001
**Title:** Add a native PTY shell backend
**Status:** NOT_STARTED

## Objective

Spawn the configured shell through a real PTY with validated cwd and inherited
Yocto environment, resize propagation, bounded buffering, and process-group
cleanup.

## Verification

```bash
cargo test -p yoctui-shell pty_backend
./scripts/test-embedded-shell.sh --backend
```

## Definition of done

- PTY shell startup, resize, output bounds, cancellation, and process-tree
  cleanup are covered by focused tests.

## Next task

After completion, select `SHELL-TERM-001`.
