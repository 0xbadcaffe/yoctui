# Current Task

## Task

**ID:** INSPECTOR-UI-001
**Title:** Redesign Inspector shell
**Status:** IN_PROGRESS

## Objective

Provide one consistent dense-but-readable Inspector structure for every
supported typed selection without moving parsing or state into widgets.

## Dependencies

- `FOUNDATION-UI-003` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-ui/src/primitives.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Typed Inspector modes have a consistent title and section order: primary
  facts, secondary facts, related paths, recent output, contextual actions.
- Existing task, recipe, layer, file, dependency, package, artifact, job,
  error, test, utility, daemon/session, and compatibility selections retain
  their authoritative content and actions.
- Missing sections are omitted or explicitly unavailable; no placeholder data
  is invented.
- Wide Inspector panels are dense and readable, and collapsed/narrow layouts
  preserve access through existing focus/navigation paths.
- Theme, no-color, and focus styling use semantic roles.

## Verification

```bash
cargo test -p yoctui-ui next_generation_inspector_shell
cargo test -p yoctui-ui inspector
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
