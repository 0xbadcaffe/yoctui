# Current Task

## Task

**ID:** COMPAT-BITBAKE-CMD-001
**Title:** Make BitBake command construction capability-aware
**Status:** IN_PROGRESS

## Objective

Audit every BitBake process invocation and require the connected environment's
central capability snapshot to select a supported typed argv implementation or
reject the action before spawn.

## Dependencies

- `COMPAT-DAEMON-001` — DONE
- `COMPAT-VERSION-001` — DONE

## Relevant files

- `crates/yoctui-bitbake/src/bitbake_cli_control.rs`
- `crates/yoctui-bitbake/src/lib.rs`
- `crates/yoctui-bitbake/src/server_controller.rs`
- `crates/yoctui-bitbake/src/signature.rs`
- `crates/yoctui-model/src/compatibility_catalog.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Every BitBake invocation and option emitted by Yoctui is inventoried and
  associated with a stable `CapabilityId` and catalog implementation.
- Command construction accepts the current normalized snapshot and selected
  implementation; it never compares release/version values locally.
- Preferred and maintained fallback implementations emit exact shell-free argv
  appropriate to their positive evidence.
- Unknown, unavailable, unsupported, stale, or implementation-mismatched
  capabilities reject before process creation with the snapshot reason.
- Tests prove old/new variants, fallback selection, unsupported-option absence,
  stale snapshot rejection, and zero spawn attempts for unavailable actions.

## Verification

```bash
cargo test -p yoctui-bitbake compatibility_command
cargo clippy -p yoctui-bitbake --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
