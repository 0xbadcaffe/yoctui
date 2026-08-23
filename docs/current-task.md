# Current Task

## Task

**ID:** RAW-RESPONSIVE-001
**Title:** Implement Raw Mode responsive layouts
**Status:** IN_PROGRESS

## Objective

Verify wide category/command/help composition, medium Inspector replacement,
narrow pane switching, dialogs, output, and too-small terminal safety.

## Dependencies

- `RAW-OUTPUT-UI-001` — DONE
- `RAW-FAVORITE-UI-001` — DONE
- `RAW-SEARCH-001` — DONE

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

- Wide, medium, narrow, and below-minimum layouts render without panic.
- Inspector replacement and pane switching preserve exact selection and focus.
- TestBackend tests cover dialogs, output, forms, Favorites, and responsive
  text at supported widths.

## Verification

```bash
cargo test -p yoctui-ui raw_responsive
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
