# Current Task

## Task

**ID:** COMPAT-PROTOCOL-001
**Title:** Add capability snapshot to client and daemon protocol
**Status:** IN_PROGRESS

## Objective

Carry the exact environment identity and generated capability snapshot from
daemon to clients as bounded, versioned, typed protocol data and update events.

## Dependencies

- `COMPAT-CAP-MODEL-001` — DONE
- `COMPAT-ENV-ID-001` — DONE

## Relevant files

- `crates/yoctui-protocol/src/daemon.rs`
- `crates/yoctui-protocol/src/lib.rs`
- `crates/yoctui-model/src/compatibility.rs`
- `docs/protocol.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Protocol DTOs carry bounded identity fields, stable capability IDs, five
  states, reason code/text, evidence, selected implementation, and generation.
- Snapshot and update messages are versioned and distinguish full replacement
  from generation-correlated change.
- Validation rejects oversized collections/text/argv, duplicate IDs, invalid
  generation, and stale/malformed data before model conversion.
- Unknown enum/message values remain forward compatible and fail closed.
- Round-trip and bound tests pass.

## Verification

```bash
cargo test -p yoctui-protocol compatibility
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
