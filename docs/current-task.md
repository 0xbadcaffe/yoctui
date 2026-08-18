# Current Task

## Task

**ID:** COMPAT-UTILITIES-001
**Title:** Correlate every Yocto utility to environment capabilities
**Status:** IN_PROGRESS

## Objective

Audit the complete utility workbench and derive every utility action and exact
implementation from the centralized daemon capability snapshot.

## Dependencies

- `COMPAT-BITBAKE-CMD-001` — DONE
- `COMPAT-BITBAKE-API-001` — DONE
- `COMPAT-DEVTOOL-001` — DONE
- `COMPAT-RECIPETOOL-001` — DONE
- `COMPAT-LAYERS-001` — DONE
- `COMPAT-PKGDATA-001` — DONE

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

- Every utility in the existing utility catalog is categorized as available,
  limited, unavailable, intentionally unsupported, or unknown from the current
  environment snapshot.
- Host PATH alone never proves compatibility with the selected build
  environment; utility previews/runs require exact tool and implementation
  authority.
- Required commands, options, metadata, artifacts, and fallbacks remain typed
  and independently testable.
- Daemon and local effect routing consume the same typed authority and do not
  reconstruct availability independently.
- Tests cover the complete utility catalog, partial/unknown environments,
  stale snapshots, exact implementation selection, and unavailable reasons.

## Verification

```bash
cargo test -p yoctui-bitbake compatibility_utilities
cargo test -p yoctui-model compatibility_utilities
./scripts/verify-roadmap.sh
```
