# Current Task

## Task

**ID:** COMPAT-BITBAKE-GETVAR-001
**Title:** Use the release-supported BitBake variable-query command
**Status:** IN_PROGRESS

## Objective

Correct the live-discovered BitBake variable-query command boundary: BitBake
2.18 provides `bitbake-getvar`; it does not support `bitbake --getvar`.

## Dependencies

- `COMPAT-BITBAKE-CMD-001` — DONE
- `COMPAT-PROBE-001` — DONE
- `COMPAT-TEST-CMDS-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility_catalog.rs`
- `crates/yoctui-bitbake/src/compatibility_probe.rs`
- `crates/yoctui-bitbake/src/compatibility_command.rs`
- `crates/yoctui-bitbake/src/compatibility_fixtures.rs`
- `docs/architecture.md`
- `docs/compatibility.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- `bitbake-getvar` is a typed initialized-environment tool identity with a safe
  direct help/options probe.
- The command planner emits exact `bitbake-getvar` argv when that implementation
  is authorized and retains the maintained `bitbake -e` fallback.
- No command path emits unsupported `bitbake --getvar`.
- Old, modern, absent-tool, stale-generation, and exact-executable tests pass;
  live Wrynose 6.0.2 evidence confirms the utility form.

## Verification

```bash
cargo test --workspace --all-features compatibility_command_getvar
cargo test --workspace --all-features compatibility_probe
./scripts/verify-roadmap.sh
```
