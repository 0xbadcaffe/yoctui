# Current Task

## Task

**ID:** RAW-CAP-PROBE-001
**Title:** Integrate Raw availability with daemon capability snapshot
**Status:** IN_PROGRESS

## Objective

Audit Raw templates against the existing capability catalog, add only the
missing safe direct option probes needed to distinguish their availability,
and publish the resulting authority through the daemon's existing snapshot.

## Dependencies

- `RAW-CAP-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-model/src/compatibility.rs`
- `crates/yoctui-model/src/compatibility_catalog.rs`
- `crates/yoctui-model/src/raw_catalog_builtin.rs`
- `crates/yoctui-bitbake/src/compatibility.rs`
- `crates/yoctui-cli/src/daemon_compatibility.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Existing safe capability records are reused wherever they fully distinguish
  a Raw command's required BitBake behavior.
- Missing option-level distinctions use bounded direct help/metadata probes;
  no probe mutates a build or invokes a shell.
- Probe results remain daemon-owned, generation-correlated, and identical for
  every attached client.
- Tests cover positive, negative, inconclusive, stale-generation, and absent-
  authority behavior across model, BitBake adapter, and daemon integration.

## Verification

```bash
cargo test -p yoctui-model raw_capability_probe
cargo test -p yoctui-bitbake raw_capability_probe
cargo test -p yoctui -- raw_capability_probe
cargo clippy -p yoctui-model --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
