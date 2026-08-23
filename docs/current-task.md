# Current Task

## Task

**ID:** RAW-HELP-UI-001
**Title:** Implement selection-following Raw command help
**Status:** IN_PROGRESS

## Objective

Replace the Raw Inspector placeholder with bounded help derived exclusively
from the exact highlighted catalog record and current capability authority.

## Dependencies

- `RAW-COMMAND-UI-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The Inspector follows the exact highlighted stable command identity and
  clears explicitly when no command is selected; search, category, catalog,
  and capability changes update it immediately without cached prose.
- Content is rendered in the specified order: exact reference description and
  section, exact template, five-state capability with authoritative reason and
  selected implementation, interaction mode, safety class, typed parameter
  definitions, and textual favorite state.
- Executable, disabled, and reference-only entries remain selectable and
  explainable; the widget never infers support or suggests that inert shell,
  filesystem, companion, or conceptual reference entries can run.
- Long Unicode descriptions, reasons, implementations, templates, and
  parameter help wrap or clip within bounds without panicking or losing
  explicit textual meaning in no-color mode.
- Wide and medium layouts show selection-following help in the Inspector;
  narrow layouts expose the same content through the existing Inspector
  overlay without changing command selection or browser focus.
- TestBackend tests cover every help field and five-state availability, exact
  selected-record following, empty/stale states, reference-only meaning,
  responsive rendering, bounds, and no-color accessibility.

## Verification

```bash
cargo test -p yoctui-ui raw_command_help
cargo clippy -p yoctui-model -p yoctui-app -p yoctui-ui --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
