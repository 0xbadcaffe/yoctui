# Current Task

## Task

**ID:** COMPAT-CATALOG-001
**Title:** Create versioned capability catalog
**Status:** IN_PROGRESS

## Objective

Create one authoritative typed and testable catalog that maps every initial
Yoctui behavior capability to its requirements, safe probes, preferred and
fallback implementations, known release boundaries, and UI reason.

## Dependencies

- `COMPAT-CAP-MODEL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility.rs`
- `docs/compatibility.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- The catalog is versioned, immutable, data-driven, and contains exactly one
  entry for every `CapabilityId`.
- Entries type required tools, commands/subcommands/options, metadata/API
  requirements, direct probes, preferred/fallback implementations, advisory
  release boundaries, and exact default unavailable reason.
- Catalog validation rejects duplicate/missing IDs, invalid requirements,
  unsafe probes, and a fallback without an explicit selector/evidence rule.
- Rendering code has no capability inventory or release table.
- Focused catalog completeness and validation tests pass.

## Verification

```bash
cargo test -p yoctui-model compatibility::catalog
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
