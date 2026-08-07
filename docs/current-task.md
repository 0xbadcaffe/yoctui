# Current Task

## Task

**ID:** RELVAL-PTY-001
**Title:** Add a real PTY-driven TUI test harness
**Status:** NOT_STARTED

## Objective

Launch the release binary in a real pseudo-terminal, feed keys and resize
events, parse ANSI/VT state into deterministic cells, and retain failure
evidence as required by `docs/ui-acceptance-tests.md`.

## Verification

```bash
cargo test -p yoctui-e2e pty_harness
./scripts/test-tui-pty.sh
```

## Definition of done

- The release binary is driven through a real PTY with semantic screen queries,
  resize handling, timeout enforcement, terminal restoration, and bounded
  failure artifacts.

## Next task

After completion, select `RELVAL-KEYMAP-001`.
