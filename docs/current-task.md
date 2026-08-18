# Current Task

## Task

**ID:** COMPAT-PROBE-001
**Title:** Implement safe capability probing
**Status:** IN_PROGRESS

## Objective

Evaluate catalog probes against the selected initialized build environment
without mutation, producing bounded typed evidence and explicit partial or
unknown outcomes.

## Dependencies

- `COMPAT-CATALOG-001` — DONE

## Relevant files

- `crates/yoctui-bitbake/src/compatibility_probe.rs`
- `crates/yoctui-bitbake/src/lib.rs`
- `crates/yoctui-model/src/compatibility_catalog.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Executable, `--version`, help/subcommand/option, metadata, backend/protocol,
  artifact, and configuration probes are typed and non-mutating.
- External probes use exact shell-free argv, a deadline, process-group
  cancellation, and bounded stdout/stderr.
- Probe failure, timeout, truncation, missing input, and partial completion
  produce explicit inconclusive/negative evidence rather than availability.
- Results are correlated to an exact environment identity and expose cache
  inputs/invalidation hooks without sharing state across environments.
- Fake-process tests cover success, missing command/option/tool, non-zero,
  timeout, oversized output, unsafe executable, and environment mismatch.

## Verification

```bash
cargo test -p yoctui-bitbake compatibility_probe
cargo clippy -p yoctui-bitbake --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
