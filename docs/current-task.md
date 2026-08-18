# Current Task

## Task

**ID:** COMPAT-DEVTOOL-001
**Title:** Make Devtool workflows capability-aware
**Status:** IN_PROGRESS

## Objective

Probe the connected environment's Devtool subcommands and options, then derive
every Devtool action and exact argv implementation from the centralized daemon
capability snapshot.

## Dependencies

- `COMPAT-BITBAKE-CMD-001` — DONE

## Relevant files

- `crates/yoctui-bitbake/src/lib.rs`
- `crates/yoctui-cli/src/daemon_devtool.rs`
- `crates/yoctui-app/src/`
- `crates/yoctui-model/src/devtool.rs`
- `crates/yoctui-model/src/compatibility_catalog.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Modify, finish, deploy-target, and upgrade subcommands/options are derived
  from the centralized capability catalog and direct initialized-environment
  probes, never host PATH or release-local checks.
- Every Devtool preview/run requires the current environment and generation,
  enabled capability, and exact selected command implementation.
- Missing subcommands/options retain exact unavailable reasons and reject
  before process creation; one available subcommand cannot authorize another.
- Daemon and local effect routing consume the same typed authority and do not
  reconstruct availability independently.
- Tests cover old/new command surfaces, missing subcommands/options, stale
  snapshots, exact argv, and zero spawn for unavailable operations.

## Verification

```bash
cargo test -p yoctui-bitbake compatibility_devtool
cargo test -p yoctui-app compatibility_devtool
./scripts/verify-roadmap.sh
```
