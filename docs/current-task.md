# Current Task

## Task

**ID:** UX-PTY-E2E-001
**Title:** Verify the built-in terminal session UX end to end
**Status:** NOT_STARTED

## Objective

Verify the complete daemon-owned terminal-session workflow through the client,
protocol, renderer, and real PTY boundaries.

## Dependencies

- `UX-TERMINAL-UX-001` — DONE
- `UX-KEYMAP-E2E-001` — DONE

## Definition of done

- Shell creation, session and split focus, writer lease/takeover, detach,
  reattach, reconnect, clean exit, process loss, and confirmed kill retain one
  daemon-owned process authority.
- Prefix commands and literal prefix forwarding cannot leak into application
  shortcuts; Unicode, bounded paste, terminal mouse reporting, copy mode,
  search, and scrollback preserve exact bytes and selection ownership.
- Terminal replica rendering, resize, zoom, cursor, dropped-history, writer and
  read-only states remain responsive, accessible, and bounded.
- Runtime, model, protocol, UI, and controlling-PTY evidence agree on terminal
  identity and lifecycle outcomes without a client-side parser or PTY process.

## Verification

```bash
cargo test -p yoctui -- ux_terminal_runtime
./scripts/test-workbench-terminal.sh
./scripts/test-tui-pty.sh
```
