# Current Task

## Task

**ID:** COMPAT-BITBAKE-API-001
**Title:** Make BitBake backend and API behavior capability-aware
**Status:** IN_PROGRESS

## Objective

Audit every BitBake backend integration and select Tinfoil, server/socket,
native-event, metadata, graph, runqueue, cancellation, reconnect, signature,
and variable behavior from the connected environment's centralized capability
snapshot.

## Dependencies

- `COMPAT-DAEMON-001` — DONE
- `COMPAT-VERSION-001` — DONE

## Relevant files

- `bridge/yoctui_bridge.py`
- `bridge/tests/`
- `crates/yoctui-bitbake/bridge/yoctui_bridge.py`
- `crates/yoctui-bitbake/src/bitbake_socket.rs`
- `crates/yoctui-bitbake/src/server_controller.rs`
- `crates/yoctui-bitbake/src/lib.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Tinfoil, server/socket, native-event, metadata, dependency graph, task and
  runqueue event, cancellation, shutdown/reconnect, signature, and variable
  integrations are inventoried against stable capability IDs.
- Backend selection consumes the current daemon snapshot and selected adapter;
  no backend or bridge module performs local release-version policy.
- Maintained API families use explicit typed adapters and safely bounded
  alternatives; absent behavior remains unavailable or unknown with evidence.
- Bridge handshakes negotiate API/event behavior directly where possible, and
  stale or mismatched capability generations are rejected.
- Tests cover materially different adapter snapshots, absent APIs/events,
  reconnect/cancellation behavior, and no unsupported backend operation.

## Verification

```bash
cargo test -p yoctui-bitbake compatibility_api
python3 -m pytest bridge/tests -k compatibility
./scripts/verify-roadmap.sh
```
