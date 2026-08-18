# Current Task

## Task

**ID:** COMPAT-WORKSPACE-CATALOG-001
**Title:** Catalog every workspace capability requirement
**Status:** IN_PROGRESS

## Objective

Inventory every Navigator destination and effect-producing action as local-only
or as an exact centralized behavior capability requirement.

## Dependencies

- `COMPAT-UTILITIES-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility.rs`
- `crates/yoctui-model/src/compatibility_catalog.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-model/src/workspace_compatibility.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Every Navigator destination and every `Effect` variant is exhaustively
  classified as client-local or assigned exact capability requirements.
- Missing behavior capabilities are added to the centralized vocabulary and
  versioned catalog rather than borrowing similarly named capabilities.
- Requirements support all-of and any-of alternatives without release checks.
- Catalog validation remains complete and every external behavior has typed
  tool/command/option/metadata/probe/implementation policy.
- Tests fail when a destination/effect is omitted and cover representative
  local, single-capability, all-of, and alternative requirements.

## Verification

```bash
cargo test -p yoctui-model compatibility_workspace_catalog
./scripts/verify-roadmap.sh
```
