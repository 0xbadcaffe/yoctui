# Current Task

## Task

**ID:** RAW-CATALOG-001
**Title:** Encode executable BitBake command surface
**Status:** IN_PROGRESS

## Objective

Encode the supplied reference's BitBake command templates and exact help text
as a versioned built-in Raw catalog, while keeping shell pipelines, conceptual
material, companion tools, and unsupported command forms reference-only.

## Dependencies

- `RAW-CATALOG-MODEL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `docs/reference/bitbake-cheatsheet-wrynose-6.0-bitbake-2.18.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Executable BitBake entries are structured typed argv templates with exact
  reference templates and descriptions.
- Shell pipelines, conceptual workflows, companion commands, and unsupported
  forms are explicitly reference-only and cannot produce argv.
- Categories follow the reference table of contents without presenting
  conceptual-only material as executable groups.
- Tests cover representative executable, interactive, destructive, joined-
  parameter, and reference-only entries plus full built-in validation.

## Verification

```bash
cargo test -p yoctui-model raw_catalog
cargo clippy -p yoctui-model --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
