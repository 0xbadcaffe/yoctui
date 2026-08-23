# Current Task

## Task

**ID:** RAW-MODEL-001
**Title:** Implement Raw Mode application state reducer and actions
**Status:** IN_PROGRESS

## Objective

Implement the pure Raw Mode application state, typed actions, and reducer
transitions that own browsing, exact selection-following help, search, forms,
preview/output/history/favorite views, and focus restoration.

## Dependencies

- `RAW-CATALOG-001` — DONE
- `RAW-CAP-001` — DONE
- `RAW-PARAM-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Raw state owns category, command, help/reference, form, preview, execution,
  history, favorite, search query/result, selection, and focus-return identity.
- Typed actions produce deterministic bounded transitions and never parse
  terminal/process text or construct backend work directly.
- Selection follows stable catalog identities across filtering and replacement;
  empty results, stale IDs, and invalid indices clamp or fail closed without a
  panic.
- Forms retain typed parameter fields and the shared expert argv editor;
  opening/cancelling preview preserves or restores the exact prior state.
- Capability authority replacement immediately reprojects availability and
  closes a now-unsafe form/preview with an exact reason and no start effect.
- Model and app tests cover normal browsing, search, selection-following help,
  form/preview transitions, empty and replacement states, favorites/history
  navigation, focus restoration, and capability invalidation.

## Verification

```bash
cargo test -p yoctui-model raw_mode
cargo test -p yoctui-app raw_mode
cargo clippy -p yoctui-model -p yoctui-app --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
