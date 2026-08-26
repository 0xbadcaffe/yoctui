# Current Task

## Task

**ID:** UX-TERMINAL-EVAL-001
**Title:** Evaluate tui-term against typed terminal replicas
**Status:** NOT_STARTED

## Objective

Prove whether `tui-term` can render daemon-owned typed terminal replicas without
introducing client-side ANSI parsing, duplicate emulation, unbounded state, or
replica drift; retain the current renderer if the adapter cannot preserve every
boundary and behavior.

## Dependencies

- `UX-SPEC-001` — DONE
- `UX-LICENSE-001` — DONE
- `PTY-MULTI-001` — DONE

## Relevant files

- daemon typed screen/cell replica and current custom renderer
- `tui-term` parser/screen ownership and adapter surface
- cursor, style, Unicode, resize, scrollback, loss, and multi-client parity
- dependency/license/MSRV/feature/binary-size evidence
- deterministic TestBackend and real PTY comparison fixtures

## Definition of done

- No adapter reparses ANSI or creates another terminal-emulation authority.
- Every typed cell, cursor/style state, resize, scrollback/loss condition, and
  multi-client snapshot renders equivalently within explicit bounds.
- Dependency features, binary size, MSRV, notices, SBOM, deny, and locked
  offline evidence are refreshed before any admission.
- A rejection records measured parity/boundary evidence and leaves the current
  renderer fully covered; adoption must pass model/UI and real PTY tests.

## Verification

```bash
cargo test -p yoctui-model ux_terminal_adapter
cargo test -p yoctui-ui ux_terminal_adapter
cargo deny check
```
