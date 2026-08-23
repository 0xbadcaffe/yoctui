# Current Task

## Task

**ID:** RAW-CAP-001
**Title:** Map Raw commands to capability requirements
**Status:** IN_PROGRESS

## Objective

Define and validate the capability requirement and projected availability
semantics for every executable Raw command using the connected environment's
authoritative capability snapshot.

## Dependencies

- `RAW-CATALOG-001` — DONE
- `COMPAT-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-model/src/compatibility.rs`
- `crates/yoctui-model/src/raw_catalog_builtin.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Every executable Raw template carries a non-empty explicit all-of or any-of
  capability requirement.
- Available, Limited, Unavailable, Unknown, and Unsupported projections retain
  exact reasons and do not infer availability from reference version text.
- Reference-only commands always project Unsupported and cannot become
  executable through capability state.
- Tests cover all requirement operators and every projected availability state.

## Verification

```bash
cargo test -p yoctui-model raw_capability
cargo clippy -p yoctui-model --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
