# Current Task

## Task

**ID:** RAW-LIVE-001
**Title:** Validate representative Raw commands against supported BitBake
**Status:** BLOCKED

## Objective

Live validation harnesses are present and namespaces now work without sudo,
but the available build directory has stale `/workspace/.yoctui-fresh-poky`
layer paths and BitBake cannot initialize.

## Dependencies

- `RAW-FAVORITE-UI-001` — DONE
- `RAW-SEARCH-001` — DONE
- `RAW-RESPONSIVE-001` — DONE
- `RAW-OUTPUT-UI-001` — DONE
- `RAW-HISTORY-001` — DONE
- `RAW-FAVORITE-UI-001` — DONE
- `RAW-SECURITY-001` — DONE
- `RAW-COMPAT-001` — DONE

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

- Recreate/correct `.yoctui-fresh-build` so its `BBLAYERS` point to the local
  Poky checkout.
- Run `YOCTUI_LIVE_RAW=1 YOCTUI_LIVE_BUILD_DIR=/path/to/build
  ./scripts/verify-live-raw-mode.sh`, then
  `./scripts/verify-raw-mode-evidence.sh`.

## Verification

```bash
./scripts/verify-live-raw-mode.sh
./scripts/verify-raw-mode-evidence.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
