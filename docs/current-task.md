# Current Task

## Task

**ID:** COMPAT-UI-001
**Title:** Expose capability state clearly in the UI
**Status:** IN_PROGRESS

## Objective

Close the complete capability-aware UI acceptance gate, including the typed
projection, dedicated inspector, and every visible action surface.

## Dependencies

- `COMPAT-UI-MODEL-001` — DONE
- `COMPAT-UI-INSPECTOR-001` — DONE
- `COMPAT-UI-ACTIONS-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility_ui.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Useful unavailable actions remain discoverable and visibly disabled with an
  exact authoritative reason or maintained alternative.
- Available, Limited, Unavailable, Unknown, and Unsupported are distinct in
  the responsive Environment/Compatibility workspace and action surfaces.
- Normal workflows avoid release-number clutter and consume only the one
  daemon-owned authority projection.
- Snapshot replacement, invalidation, responsive bounds, themes, and no-color
  mode remain safe without stale widget state or invalid launches.
- The aggregate UI/app compatibility suites and PTY snapshots pass.

## Verification

```bash
cargo test -p yoctui-ui compatibility
cargo test -p yoctui-app compatibility_ui
./scripts/test-tui-snapshots.sh
./scripts/verify-roadmap.sh
```
