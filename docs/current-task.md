# Current Task

## Task

**ID:** COMPAT-DOCTOR-001
**Title:** Add compatibility report to Doctor diagnostics
**Status:** IN_PROGRESS

## Objective

Extend `yoctui doctor` with a bounded human-readable and machine-readable
report of the daemon-owned connected-environment compatibility authority.

## Dependencies

- `COMPAT-UI-001` — DONE
- `COMPAT-DAEMON-001` — DONE

## Relevant files

- `crates/yoctui-cli/src/main.rs`
- `crates/yoctui-protocol/src/lib.rs`
- `crates/yoctui-protocol/src/daemon_ipc.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Doctor reports authoritative Yocto/OE-Core/Poky identity, BitBake,
  backend/protocol, build directory, and support classification.
- It summarizes all five capability states and lists missing tools, limited,
  unsupported, and unknown features with exact reasons.
- A bounded structured output mode serializes the same typed authority without
  independent probing, inference, unbounded collections, or raw process text.
- Disconnected, missing, malformed, and stale daemon authority fail closed and
  remain diagnostically distinct.
- Existing environment and bridge diagnostics remain available.

## Verification

```bash
cargo test -p yoctui doctor_compatibility
cargo test -p yoctui-protocol compatibility
./scripts/verify-roadmap.sh
```
