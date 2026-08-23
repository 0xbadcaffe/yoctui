# Current Task

## Task

**ID:** RAW-CATALOG-TRACE-001
**Title:** Verify Raw catalog traceability to bundled reference
**Status:** IN_PROGRESS

## Objective

Verify that every generated Raw entry remains traceable to the immutable
reference snapshot with exact command text and descriptions, and that the
checked-in generated catalog cannot drift from its deterministic source.

## Dependencies

- `RAW-CATALOG-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-model/src/raw_catalog_builtin.rs`
- `docs/reference/bitbake-cheatsheet-wrynose-6.0-bitbake-2.18.md`
- `scripts/generate-raw-catalog.py`
- `scripts/verify-raw-catalog.sh`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Every bash-block command and its adjacent description maps to exactly one
  stable source-line reference identity.
- Category headings, command text, descriptions, classifications, and
  placeholder agreement are checked independently.
- The reference hash and generated file freshness fail closed on drift.
- A repository verification script runs the deterministic generator check and
  focused model traceability tests.

## Verification

```bash
cargo test -p yoctui-model raw_catalog_trace
./scripts/verify-raw-catalog.sh
cargo clippy -p yoctui-model --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
