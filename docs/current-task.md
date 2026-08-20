# Current Task

## Task

**ID:** SYSTEM-UI-001
**Title:** Redesign System Status pane
**Status:** IN_PROGRESS

## Objective

Render a dense, consistent System Status pane from authoritative model state,
with responsive labels and exact unavailable behavior for facts the current
protocol or host sampler does not supply.

## Dependencies

- `INSPECTOR-UI-001` — DONE
- `METRICS-MODEL-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Daemon connection, uptime, BitBake lifecycle, active jobs, terminal
  sessions, and connected clients render from typed current state.
- Workspace/build identity and exact build-filesystem context render only when
  authoritative.
- Compatibility state names current, degraded, synchronizing, stale, and
  unavailable authority without inventing a daemon version or PID.
- Wide and compact presentations use consistent aligned labels and retain
  meaningful text in high-contrast and no-color modes.
- Missing values are explicitly unavailable and stale authority is never
  presented as current.
- The pane remains bounded and safe at responsive breakpoints.

## Verification

```bash
cargo test -p yoctui-ui next_generation_system_status
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
