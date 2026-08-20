# Current Task

## Task

**ID:** SYSTEM-UI-002
**Title:** Add health and warning indicators
**Status:** IN_PROGRESS

## Objective

Add compact semantic text-and-symbol health indicators to System Status and
the persistent shell for authoritative backend, disk, compatibility, log, and
workspace conditions.

## Dependencies

- `SYSTEM-UI-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Backend disconnected, synchronizing/reconnecting, and stale states use
  distinct text markers and semantic roles.
- Low build-filesystem capacity is derived only from the authoritative sample
  and uses documented warning/error thresholds.
- Degraded compatibility, log pressure/eviction, and unknown workspace state
  are named without relying on color.
- Healthy, warning, error, and unavailable conditions share reusable semantic
  presentation rather than widget-local hardcoded colors.
- Indicators remain meaningful in high-contrast and no-color modes and do not
  animate under reduced motion.
- Responsive layouts bound or prioritize status without overlap or panic.

## Verification

```bash
cargo test -p yoctui-ui next_generation_health_indicators
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
