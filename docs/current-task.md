# Current Task

## Task

**ID:** COMPAT-TEST-FIXTURES-001
**Title:** Add release capability fixtures
**Status:** IN_PROGRESS

## Objective

Add deterministic capability fixtures for representative Yocto/BitBake
generations without converting fixture coverage into release support claims.

## Dependencies

- `COMPAT-OLD-001` — DONE
- `COMPAT-UNKNOWN-001` — DONE
- `COMPAT-CATALOG-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility.rs`
- `crates/yoctui-bitbake/src/compatibility_resolver.rs`
- `crates/yoctui-bitbake/src/compatibility_version.rs`
- `crates/yoctui-bitbake/tests/`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Fixtures cover the policy's oldest representative generation, one
  intermediate generation, current stable representative, latest-known
  representative, and a synthetic future/unknown generation.
- Each fixture carries exact authoritative identity, direct observations, and
  expected capability/implementation differences.
- Future/unknown behavior enables only positive direct probes; conservative
  fallback behavior remains exact for older/intermediate fixtures.
- Fixtures are deterministic, bounded, centralized, and reusable by command
  and UI tests; labels explicitly deny live/support evidence status.
- Exact fixture differences are asserted and catalog completeness is retained.

## Verification

```bash
cargo test --workspace --all-features compatibility_fixture
./scripts/verify-roadmap.sh
```
