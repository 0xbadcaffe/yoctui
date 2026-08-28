# Current Task

## Task

**ID:** UX-CONCEPT-TERMINAL-LIVE-001
**Title:** Drive the Terminal Sessions concept through a real client
**Status:** IN_PROGRESS

## Objective

Drive a real client to Terminal Sessions with Ctrl+B t, prove the daemon-owned
split panes and writer/read-only states, and retain visible prefix-help,
scrollback-search, match, and dropped-history evidence.

## Dependencies

- UX-CONCEPT-ACCEPTANCE-001 — DONE

## Definition of done

- Ctrl+B t reaches Terminal Sessions in the real-client harness.
- Two daemon-owned split panes expose writer/read-only ownership.
- Prefix help remains visible and owns the prefixed input sequence.
- Search match and dropped-history counts are asserted.
- Focused runtime and workbench terminal tests pass.

## Verification

```bash
cargo test -p yoctui -- ux_terminal_runtime
./scripts/test-workbench-terminal.sh
```
