# Current Task

## Task

**ID:** COMPAT-UI-NAV-ACTIONS-001
**Title:** Render capability state in global action surfaces
**Status:** IN_PROGRESS

## Objective

Project centralized compatibility state into Navigator destinations,
command-palette operations, and contextual footers without preventing users
from navigating to inspect unavailable functionality.

## Dependencies

- `COMPAT-UI-MODEL-001` — DONE
- `COMPAT-UI-INSPECTOR-001` — DONE
- `COMPAT-UI-ACTION-CATALOG-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility_ui.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Navigator destinations remain selectable and show concise five-state
  compatibility status from the centralized destination projection.
- Command-palette entries derive enabled/disabled/limited state from the typed
  command definition plus ordinary selection prerequisites.
- Disabled operations remain selectable for exact reason inspection but cannot
  activate; navigation commands remain usable for discoverability.
- Limited operations show the selected fallback/limitation without release
  number clutter; local commands are unaffected by missing authority.
- Contextual footers show only relevant capability status and exact reasons are
  available in the persistent Inspector/palette detail.
- Live snapshot replacement updates all global surfaces with no stale cache,
  invalid launch, selection loss, shortcut leakage, or panic.

## Verification

```bash
cargo test -p yoctui-ui compatibility_ui_nav_actions
cargo test -p yoctui-app compatibility_ui_nav_actions
./scripts/test-tui-snapshots.sh
./scripts/verify-roadmap.sh
```
