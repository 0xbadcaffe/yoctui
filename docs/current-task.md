# Current Task

## Task

**ID:** VISUAL-TEST-001
**Title:** Create semantic Ratatui snapshot tests
**Status:** IN_PROGRESS

## Objective

Create stable semantic TestBackend snapshots for every required next-generation
workspace and representative typed dialogs. Assert meaningful regions, state,
selection, and controls without coupling the suite to irrelevant whitespace.

## Dependencies

- `RESPONSIVE-UI-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Semantic snapshots cover Tasks, Logs, Jobs, Recipes, Layers, Images,
  Dashboard, Settings, Build Environment, and Terminal/session views.
- Representative standard, confirmation, destructive, result, and editor
  dialogs retain typed state, availability, validation, and controls.
- Assertions compare stable semantic regions/cells or normalized lines rather
  than incidental full-buffer whitespace.
- Snapshot fixtures are deterministic and explain intentional update review.

## Verification

```bash
cargo test -p yoctui-ui semantic_snapshots
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
