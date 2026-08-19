# Current Task

## Task

**ID:** COMPAT-PROBE-AGGREGATION-001
**Title:** Require complete direct evidence for compound capabilities
**Status:** IN_PROGRESS

## Objective

Prevent a partial direct probe from enabling a compound catalog capability.
Every required executable, subcommand, option, metadata, and backend probe must
contribute to one conservative typed decision.

## Dependencies

- `COMPAT-CATALOG-001` — DONE
- `COMPAT-PROBE-001` — DONE
- `COMPAT-DAEMON-RUNTIME-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility_catalog.rs`
- `crates/yoctui-bitbake/src/compatibility_resolver.rs`
- `crates/yoctui-bitbake/src/compatibility_version.rs`
- `crates/yoctui-bitbake/src/compatibility_fixtures.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- A compound capability resolves Available only when every required direct
  probe is positive.
- Any authoritative negative required probe makes the capability Unavailable;
  mixed contradictory evidence remains an explicit conflict.
- Inconclusive or absent required evidence cannot be masked by executable
  discovery and remains disabled with an exact Unknown/limited reason.
- Centralized version fallback remains eligible only for catalog entries that
  explicitly declare it and whose direct behavior probe is genuinely
  uncollectable, never when a required direct probe failed.
- Tests cover positive, negative, inconclusive, contradictory, future-version,
  subcommand-timeout, and option-timeout combinations without spawning a real
  utility.

## Verification

```bash
cargo test -p yoctui-bitbake compatibility_probe_aggregation
cargo test -p yoctui-bitbake compatibility_future_unknown
./scripts/verify-roadmap.sh
```
