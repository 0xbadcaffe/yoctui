# Current Task

## Task

**ID:** RAW-RECIPE-001
**Title:** Integrate authoritative Raw selectors
**Status:** IN_PROGRESS

## Objective

Project recipe, image, target, task, and multiconfig choices from existing
authoritative model inventories into Raw parameter fields while preserving the
same validated manual-entry boundary where BitBake permits it.

## Dependencies

- `RAW-PARAM-001` — DONE
- `RECIPES-META-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Recipe and image choices come only from the current typed inventories and
  retain their exact identities.
- Target choices reuse current/recent authoritative targets without treating a
  missing inventory as an empty valid selection.
- Task choices are correlated to the exact selected recipe metadata; a stale
  recipe response cannot populate another selection.
- Multiconfig choices use authoritative configured identities when present.
- Manual target/task entry remains available only for definitions whose
  BitBake template permits it and passes `RAW-PARAM-001` validation.
- Model and app tests cover selection, absent/empty/stale inventories, manual
  entry, and identity-preserving replacement.

## Verification

```bash
cargo test -p yoctui-model raw_selector
cargo test -p yoctui-app raw_selector
cargo clippy -p yoctui-model -p yoctui-app --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
