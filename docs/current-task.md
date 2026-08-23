# Current Task

## Task

**ID:** RAW-SEARCH-001
**Title:** Search Raw categories commands and descriptions
**Status:** IN_PROGRESS

## Objective

Provide bounded case-insensitive typed search across Raw categories, commands,
and descriptions with synchronized exact selection and no-match behavior.

## Dependencies

- `RAW-COMMAND-UI-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/architecture.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Search input is bounded, case-insensitive, and typed; it matches category,
  command identity, labels, and descriptions without parsing rendered text.
- Selection remains an exact catalog identity as results change, with clear
  empty/no-match behavior and Ctrl+U clearing the query.
- Model, app, and TestBackend tests cover editing, matching, selection sync,
  empty results, bounds, and narrow responsive output.

## Verification

```bash
cargo test -p yoctui-model raw_search
cargo test -p yoctui-app raw_search
cargo test -p yoctui-ui raw_search
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
