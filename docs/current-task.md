# Current Task

## Task

**ID:** COMPAT-UI-INSPECTOR-001
**Title:** Render the Environment and Compatibility inspector
**Status:** IN_PROGRESS

## Objective

Add the specified first-class Compatibility destination and responsive
Environment/Compatibility workspace using only the typed presentation model.

## Dependencies

- `COMPAT-UI-MODEL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Compatibility is reachable from Navigator and command palette without
  emitting an environment effect or probe.
- Wide layout renders authoritative identity/summary, the filtered capability
  table, and exact selected details/evidence in the persistent Inspector.
- Medium uses the standard Inspector overlay; narrow uses the shared pane
  switcher; below 80x24 retains the resize message.
- Keys `1`-`5`, `/`, `Esc`, arrows, and `j`/`k` follow the UI specification and
  do not leak through search/focus routing.
- Absent/current/limited/unavailable/unknown/unsupported states, long content,
  every theme, no-color, and snapshot replacement render without panic.
- The global wide F1-F10 rail and canonical Tasks golden remain unchanged.

## Verification

```bash
cargo test -p yoctui-ui compatibility_ui_inspector
cargo test -p yoctui-app compatibility_ui_inspector
./scripts/test-tui-snapshots.sh
./scripts/verify-roadmap.sh
```
