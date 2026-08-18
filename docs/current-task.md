# Current Task

## Task

**ID:** COMPAT-PKGDATA-001
**Title:** Make oe-pkgdata-util capability-aware
**Status:** IN_PROGRESS

## Objective

Probe the connected environment's oe-pkgdata-util commands and generated
pkgdata, then derive every package-data action and exact argv implementation
from the centralized daemon capability snapshot.

## Dependencies

- `COMPAT-BITBAKE-CMD-001` — DONE

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

- Tool availability, individual commands, and generated pkgdata are detected
  independently, never inferred from host PATH or a release-local check.
- Every oe-pkgdata-util preview/run requires the current environment and generation,
  enabled capability, and exact selected command implementation.
- Tool unavailable, command unavailable, pkgdata not generated, and a valid
  query with no result remain distinct typed outcomes.
- Daemon and local effect routing consume the same typed authority and do not
  reconstruct availability independently.
- Tests cover old/new command surfaces, missing subcommands/options, stale
  snapshots, exact argv, and zero spawn for unavailable operations.

## Verification

```bash
cargo test -p yoctui-bitbake compatibility_pkgdata
cargo test -p yoctui-app compatibility_pkgdata
./scripts/verify-roadmap.sh
```
