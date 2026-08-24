# Current Task

## Task

**ID:** LIVE-UI-POKY-001
**Title:** Validate redesigned UI against real Poky
**Status:** BLOCKED

## Objective

The Raw workbench is complete. Global completion is blocked by the separate
real-Poky redesigned-UI evidence task; this host currently denies the
unprivileged user namespace required by BitBake, so the live harness exits
before evidence capture.

## Dependencies

- `RAW-FAVORITE-UI-001` — DONE
- `RAW-SEARCH-001` — DONE
- `RAW-RESPONSIVE-001` — DONE
- `VISUAL-TEST-003` — DONE
- `PTY-UI-TEST-001` — DONE
- `PERF-UI-002` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/architecture.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Run `unshare -Ur true` and the live UI evidence harness commands.
- If the host prerequisite remains unavailable, retain `BLOCKED` and record
  the exact reproduction rather than claiming live evidence.

## Verification

```bash
unshare -Ur true
YOCTUI_POKY_SOURCE="$PWD/.yoctui-fresh-poky" ./scripts/test-live-next-generation-ui.sh
./scripts/verify-next-generation-ui-evidence.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
