# Current Task

## Task

**ID:** UI-LITERAL-NAV-001
**Title:** Implement the mixed typed project Navigator
**Status:** IN_PROGRESS

## Objective

Replace the abstract grouped destination list in the canonical workbench with
the reference's mixed Layers, Recipes, Images, Tasks, and Targets project tree.

## Dependencies

- `UI-LITERAL-SHELL-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/tests/golden/literal-reference-160x48.cells`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Wide canonical rendering shows Layers, Recipes, Images, Tasks, and Targets.
- Layer, recipe, image, and target children come only from typed model state.
- Task-family children activate their real workspace destinations.
- Selection remains bounded, visible, and uses a full-width blue row.
- Medium and narrow layouts preserve access to the complete destination set.

## Verification

```bash
cargo test -p yoctui-model navigator
cargo test -p yoctui-app navigator
cargo test -p yoctui-ui literal_navigator
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
