# Current Task

## Task

**ID:** A11Y-UI-001
**Title:** Improve accessibility
**Status:** IN_PROGRESS

## Objective

Guarantee that the redesigned shell remains understandable and operable in
high-contrast, no-color, and reduced-motion modes, with visible focus, textual
state meaning, terminal-reader-friendly labels, and numeric progress
equivalents.

## Dependencies

- `TASKS-UI-003` — DONE
- `LOG-UI-002` — DONE
- `DIALOG-UI-001` — DONE
- `METRICS-UI-006` — DONE
- `FOOTER-UI-002` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-ui/src/primitives.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- No task, log, job, dialog, health, or compatibility state relies on color
  alone; each has a stable textual marker or label.
- High-contrast and no-color palettes preserve visible focus, selection,
  disabled state, severity, and active/inactive borders.
- Reduced motion removes animated state changes while preserving explicit
  running/pending text and stable indeterminate meaning.
- Progress bars and gauges retain numeric or textual equivalents at every
  supported breakpoint.
- Section titles, action labels, paths, status text, and progress descriptions
  remain meaningful in terminal buffer text for terminal readers.
- Wide, medium, narrow, and minimum modes do not hide the sole focus cue or
  encode an unavailable action as enabled.

## Verification

```bash
cargo test -p yoctui-ui accessibility_invariants
cargo test -p yoctui-model reduced_motion
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
