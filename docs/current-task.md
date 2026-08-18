# Current Task

## Task

**ID:** COMPAT-OLD-001
**Title:** Define older-release degradation policy
**Status:** IN_PROGRESS

## Objective

Encode and test the policy that an older supported environment preserves every
safe capability, uses only maintained catalog fallbacks, and disables isolated
newer behavior without taking down the application.

## Dependencies

- `COMPAT-VERSION-001` — DONE

## Relevant files

- `crates/yoctui-bitbake/src/compatibility_resolver.rs`
- `crates/yoctui-model/src/compatibility.rs`
- `docs/compatibility.md`
- `docs/compatibility-matrix.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Older environments produce a complete mixed-state snapshot, not a global
  unsupported/failure state.
- Positive core behavior stays enabled; maintained fallbacks are limited and
  explained; absent newer behavior is disabled with exact reasons.
- Unsupported and Unknown stay distinct from environmental Unavailable.
- No minimum supported release is claimed before live older-release evidence.
- Deterministic tests cover preserved core, fallback, newer unavailable,
  unsupported, and whole-application continuity.

## Verification

```bash
cargo test -p yoctui-model compatibility_older_release
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
