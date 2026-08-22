# Current Task

## Task

**ID:** RAW-CATALOG-MODEL-001
**Title:** Define typed Raw command category and parameter model
**Status:** IN_PROGRESS

## Objective

Define the bounded pure domain vocabulary for Raw Mode categories, commands,
reference traceability, parameters, capability requirements, interaction
modes, and safety classes.

## Dependencies

- `RAW-SPEC-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

 - Stable bounded category, command, and reference identities normalize.
 - Parameter kinds, required/optional placeholders, interaction mode, and
   safety class are closed typed values.
 - Catalog validation rejects duplicate/invalid identities, missing text,
   placeholder disagreement, unsafe templates, and missing execution policy.
 - Unit tests cover valid, partial, duplicate, oversized, and unsafe records.

## Verification

```bash
cargo test -p yoctui-model raw_catalog_model
cargo clippy -p yoctui-model --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
