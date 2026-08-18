# Current Task

## Task

**ID:** COMPAT-WORKSPACE-APP-001
**Title:** Install and enforce workspace capability authority
**Status:** IN_PROGRESS

## Objective

Install the bounded daemon wire snapshot into client model state and make all
interactive daemon/local effect routing consume the same typed authority.

## Dependencies

- `COMPAT-WORKSPACE-MODEL-001` — DONE
- `COMPAT-PROTOCOL-001` — DONE

## Relevant files

- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `crates/yoctui-model/src/workspace_compatibility.rs`
- `crates/yoctui-protocol/src/daemon.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Valid bounded wire snapshots convert once into normalized model authority;
  unknown wire values fail closed and cannot invent support.
- Attach/reconnect/update install the same snapshot into `App`; disconnect or
  absent authority invalidates it without changing presentation state.
- Interactive action routing uses the capability-aware reducer boundary for
  both daemon and local execution paths.
- Daemon-owned probe effects are not independently executed by clients.
- Unavailable/unknown/unsupported/stale actions produce no process/job effect
  and retain exact model reasons.
- Tests cover attach, update, disconnect, malformed/unknown data, local effect,
  unavailable effect, and no-spawn routing.

## Verification

```bash
cargo test -p yoctui-app compatibility_workspace_app
cargo test -p yoctui compatibility_workspace_app
./scripts/verify-roadmap.sh
```
