# Current Task

## Task

**ID:** LIVE-UI-POKY-001
**Title:** Validate redesigned UI against real Poky
**Status:** IN_PROGRESS

## Objective

The Raw workbench is complete. Global completion is waiting on the separate
real-Poky redesigned-UI evidence task. The host now permits `unshare -Ur true`,
and the storage quota blocker is gone: an isolated retry crossed 5.6 GiB and
completed `ncurses-native`, the prior failure point. The retry instead exposed
a Poky/pseudo failure in `sysvinit-inittab:do_install` while `tail` reads its
pipeline input (`couldn't allocate absolute path for ''`), after which the
BitBake server retains a client descriptor and the Yoctui job remains Running.
The harness therefore cannot yet produce its completion manifest.

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
- Reproduce and resolve the `sysvinit-inittab:do_install` pseudo failure without
  weakening the real-Poky acceptance target.
- Ensure a failed BitBake worker reaches a typed terminal job state instead of
  leaving the daemon job Running indefinitely.
- Retry with a unique `YOCTUI_NEXT_UI_EVIDENCE` directory and retain sufficient
  cold-build storage.

## Verification

```bash
unshare -Ur true
YOCTUI_POKY_SOURCE="$PWD/.yoctui-fresh-poky" ./scripts/test-live-next-generation-ui.sh
./scripts/verify-next-generation-ui-evidence.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
