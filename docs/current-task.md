# Current Task

## Task

**ID:** RAW-MOUSE-001
**Title:** Add first-class Raw Mode mouse behavior
**Status:** IN_PROGRESS

## Objective

Use shared rendered geometry for category, command, favorite, history, and
field selection with wheel parity while dialogs and PTYs retain their traps.

## Dependencies

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

- Mouse hit-testing uses the same bounded row geometry as keyboard navigation.
- Wheel and click actions preserve typed identities and focus traps.
- App and TestBackend tests cover categories, commands, Favorites, history,
  form fields, narrow bounds, and modal/PTY isolation.

## Verification

```bash
cargo test -p yoctui-app raw_mouse
cargo test -p yoctui -- raw_mouse
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
