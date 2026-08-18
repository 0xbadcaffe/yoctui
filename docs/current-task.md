# Current Task

## Task

**ID:** COMPAT-DAEMON-001
**Title:** Move compatibility state into daemon ownership
**Status:** IN_PROGRESS

## Objective

Make the persistent daemon the sole owner of the exact environment capability
snapshot so every attached client observes the same generation and reconnects
without independently inferring release support.

## Dependencies

- `COMPAT-CACHE-001` — DONE
- `COMPAT-PROTOCOL-001` — DONE
- `COMPAT-VERSION-001` — DONE

## Relevant files

- `crates/yoctui-model/src/daemon_state.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-cli/src/daemon.rs`
- `crates/yoctui-cli/src/client_transport.rs`
- `crates/yoctui-bitbake/src/compatibility_cache.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Daemon state owns exactly one capability cache and normalized snapshot for
  the selected build environment.
- Environment/workspace changes invalidate and reprobe through a newer
  generation; stale probe results cannot replace current state.
- Attach and reconnect snapshots expose the daemon-owned compatibility state,
  and all clients observe identical generations and update events.
- Client/model replicas accept typed snapshots but never perform version or
  release inference independently.
- Tests cover initial attach, multi-client consistency, reconnect, invalidation,
  stale reprobe rejection, and environment isolation.

## Verification

```bash
cargo test -p yoctui daemon_compatibility
cargo test -p yoctui-model daemon_compatibility
./scripts/verify-roadmap.sh
```
