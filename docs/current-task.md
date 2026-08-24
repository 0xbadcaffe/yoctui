# Current Task

## Task

**ID:** LIVE-UI-POKY-001
**Title:** Validate redesigned UI against real Poky
**Status:** BLOCKED

## Objective

The Raw workbench is complete. Global completion is blocked by the separate
real-Poky redesigned-UI evidence task. The host now permits the required user
namespace and the daemon completes cold compatibility startup within the
extended bounded deadline. A single isolated, two-thread Poky build reaches
native task execution but fails with `EDQUOT` (`Disk quota exceeded`) before it
can write the evidence manifest.

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
- Capture and verify the required real-Poky evidence manifest.
- Before retrying, provide enough project quota for the cold Poky build (the
  isolated run failed after 4 GiB of temporary build output despite free blocks
  and inodes), then use a unique `YOCTUI_NEXT_UI_EVIDENCE` directory.

## Verification

```bash
unshare -Ur true
YOCTUI_POKY_SOURCE="$PWD/.yoctui-fresh-poky" ./scripts/test-live-next-generation-ui.sh
./scripts/verify-next-generation-ui-evidence.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
