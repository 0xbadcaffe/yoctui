# Current Task

## Task

**ID:** UI-CLEANUP-001
**Title:** Refactor rendering modules after behavior stabilizes
**Status:** IN_PROGRESS

## Objective

Split the stable next-generation renderer into explicit
shell/widgets/workspaces/dialogs/telemetry/theme/layout modules without
changing typed behavior, golden output, accessibility, or responsive geometry.

## Dependencies

- `README-UI-001` — DONE
- `UI-REGRESSION-001` — DONE
- `LIVE-UI-POKY-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-ui/src/`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Establish the required rendering-module boundaries.
- Preserve the public UI API and all typed input/rendering behavior.
- Keep visual snapshots, responsive layouts, and golden output unchanged.
- Pass UI tests, strict UI clippy, roadmap validation, and the parent UI gate.

## Verification

```bash
cargo test -p yoctui-ui
cargo clippy -p yoctui-ui --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
