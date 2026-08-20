# Current Task

## Task

**ID:** METRICS-UI-006
**Title:** Create responsive telemetry strip
**Status:** IN_PROGRESS

## Objective

Compose the authoritative CPU, RAM, build-filesystem, disk-I/O, and network
widgets into an explicit responsive telemetry strip whose pane priority and
content remain useful from wide through narrow terminals.

## Dependencies

- `METRICS-UI-005` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Wide layout composes CPU, RAM, Build FS, Read, Write, RX, and TX in stable
  terminal-native cells with bounded histories where supported.
- Medium layout preserves CPU, RAM, Build FS, and a compact I/O summary.
- Narrow layout presents an honest compact summary or hides the strip behind
  the existing status/Inspector path according to the documented pane
  priority.
- Optional unsupported metrics never reserve misleading data or display a
  synthetic zero.
- Breakpoint selection is deterministic, uses reusable layout primitives, and
  does not overlap or panic below the preferred width.
- High-contrast, no-color, and reduced-motion presentations retain meaningful
  text and visible state.

## Verification

```bash
cargo test -p yoctui-ui next_generation_telemetry_strip
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
