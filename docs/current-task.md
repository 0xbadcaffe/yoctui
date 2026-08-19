# Current Task

## Task

**ID:** FOUNDATION-UI-003
**Title:** Unify visual theme semantics
**Status:** IN_PROGRESS

## Objective

Extend the semantic theme model used by the shared primitives and workspaces
without hardcoding widget-specific colors or breaking any existing theme.

## Dependencies

- `FOUNDATION-UI-002` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-ui/src/primitives.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Every requested visual meaning maps through one semantic theme role.
- No workspace renderer hardcodes a role color.
- Every existing theme remains valid and visually distinct.
- High contrast and no-color preserve status, focus, and selection meaning.
- Semantic theme tests and the reviewed literal golden pass.

## Verification

```bash
cargo test -p yoctui-ui semantic_theme
cargo test -p yoctui-model theme
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
