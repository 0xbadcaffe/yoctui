# Current Task

## Task

**ID:** LIVE-UI-POKY-001
**Title:** Validate redesigned UI against real Poky
**Status:** IN_PROGRESS

## Objective

The Raw workbench is complete. Global completion is waiting on the separate
real-Poky redesigned-UI evidence task. The host permits `unshare -Ur true`, and
`/` is an ext4 mount without user quotas, so the attempted `setquota` command
cannot apply and quota is not the blocker. The later
`sysvinit-inittab:do_install` pseudo symptom was reproduced only on the
unsupported Ubuntu 26.04 host, whose glibc 2.43 is newer than Poky 5.2.4's
uninative glibc 2.42. A supported Ubuntu 24.04/glibc 2.39 container has crossed
both prior failure points and is running the full `core-image-minimal` evidence
harness to completion.

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
- Complete the build on a Poky-supported host without weakening the real-Poky
  acceptance target.
- Prove the harness's intentional missing target reaches a typed `Failed` job
  state instead of leaving the daemon job `Running` indefinitely.
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
