# Current Task

## Task

**ID:** RAW-PREVIEW-001
**Title:** Implement exact executable and indexed argv preview
**Status:** IN_PROGRESS

## Objective

Build the immutable Raw execution preview from one validated catalog template,
typed parameter values, validated additional argv, and the exact current
environment/capability authority.

## Dependencies

- `RAW-ARG-001` — DONE
- `RAW-CAP-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The preview reconstructs one executable and each argv element independently
  from the catalog template, typed values, and validated additional arguments.
- Required, optional, joined, composed, and explicit empty template arguments
  retain exact native argv semantics and reject missing, extra, or mismatched
  parameter values with typed errors.
- Preview rows are indexed and retain executable identity, command/catalog
  revision, capability generation, environment/build-directory identity,
  interaction mode, safety class, and exact limitations.
- A stale or unavailable capability authority cannot produce a preview.
- No model or widget API exposes a joined command string as execution
  authority.
- Model and TestBackend UI tests cover representative templates, optional and
  empty values, Unicode, additional argv, stale authority, indexed rendering,
  and narrow-terminal degradation.

## Verification

```bash
cargo test -p yoctui-model raw_preview
cargo test -p yoctui-ui raw_preview
cargo clippy -p yoctui-model -p yoctui-ui --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
