# Current Task

## Task

**ID:** COMPAT-LAYERS-001
**Title:** Make bitbake-layers workflows capability-aware
**Status:** IN_PROGRESS

## Objective

Probe the connected environment's bitbake-layers subcommands and options, then
derive every Layers action and exact argv implementation from the centralized
daemon capability snapshot.

## Dependencies

- `COMPAT-BITBAKE-CMD-001` — DONE
- `COMPAT-BITBAKE-API-001` — DONE

## Relevant files

- `crates/yoctui-bitbake/src/lib.rs`
- `crates/yoctui-cli/src/`
- `crates/yoctui-app/src/`
- `crates/yoctui-model/src/`
- `crates/yoctui-model/src/compatibility_catalog.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Show-layers, create-layer, add-layer, remove-layer, and every existing Layers
  action are derived from centralized direct initialized-environment probes,
  never host PATH or release-local checks.
- Every bitbake-layers preview/run requires the current environment and generation,
  enabled capability, and exact selected command implementation.
- Missing subcommands/options retain exact unavailable reasons and reject
  before process creation; one available subcommand cannot authorize another.
- Daemon and local effect routing consume the same typed authority and do not
  reconstruct availability independently.
- Tests cover old/new command surfaces, missing subcommands/options, stale
  snapshots, exact argv, and zero spawn for unavailable operations.

## Verification

```bash
cargo test -p yoctui-bitbake compatibility_layers
cargo test -p yoctui-app compatibility_layers
./scripts/verify-roadmap.sh
```
