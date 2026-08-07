# Current Task

## Task

**ID:** RELVAL-KEYMAP-001
**Title:** Verify every documented keyboard shortcut
**Status:** NOT_STARTED

## Objective

Generate and drive the documented keyboard matrix through the real PTY,
checking valid transitions and inert behavior in invalid contexts.

## Verification

```bash
cargo test -p yoctui-e2e keyboard_matrix
./scripts/test-tui-keymap.sh
```

## Definition of done

- Every documented shortcut has an executable path and context-invalid keys
  remain inert with visible disabled explanations.

## Next task

After completion, select `RELVAL-FLOW-001`.
