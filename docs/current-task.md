# Current Task

## Task

**ID:** COMPAT-CACHE-001
**Title:** Cache and invalidate capability snapshots safely
**Status:** IN_PROGRESS

## Objective

Associate cached capability state with one exact build-environment fingerprint
and invalidate it on every relevant workspace, BitBake, environment, layer, or
daemon-workspace change without leaking state between projects.

## Dependencies

- `COMPAT-PROBE-001` — DONE
- `COMPAT-ENV-ID-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility.rs`
- `crates/yoctui-bitbake/src/compatibility_cache.rs`
- `crates/yoctui-bitbake/src/lib.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- A deterministic fingerprint includes exact workspace/build/source,
  BitBake/tool, layer-series/configuration, initialized environment, and
  backend/protocol identity.
- Cache lookup requires an exact fingerprint and returns no cross-project data.
- Relevant identity/configuration/reconnect changes invalidate and advance the
  snapshot generation; unchanged identity may reuse bounded probe results.
- Stale generations and overflow fail closed.
- Tests cover each invalidation dimension, reuse, project isolation, and
  generation behavior.

## Verification

```bash
cargo test -p yoctui-model compatibility_cache
cargo test -p yoctui-bitbake compatibility_cache
./scripts/verify-roadmap.sh
```
