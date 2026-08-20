# Current Task

## Task

**ID:** HEADER-UI-001
**Title:** Redesign header
**Status:** IN_PROGRESS

## Objective

Redesign the compact persistent header around the authoritative identity and
build-state priority order, with deterministic progressive hiding at reduced
widths.

## Dependencies

- `SYSTEM-UI-002` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Project/Yoctui identity remains first, followed by build state and target.
- MACHINE, DISTRO/release, daemon state, and BitBake state render only from
  authoritative model values in the documented priority order.
- Wide, medium, and narrow variants hide lower-priority fields
  deterministically without clipping higher-priority state.
- Daemon and BitBake reuse the shared text-marker health semantics and stale
  authority never appears current.
- No daemon version, PID, target, release, or environment value is inferred.
- High-contrast, no-color, reduced-motion, and minimum-width layouts remain
  readable and panic-free.

## Verification

```bash
cargo test -p yoctui-ui next_generation_header
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
