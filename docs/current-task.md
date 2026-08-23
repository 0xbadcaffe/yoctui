# Current Task

## Task

**ID:** RAW-DOC-001
**Title:** Document Raw Mode operation and safety
**Status:** IN_PROGRESS

## Objective

Document browsing, parameters, exact argv, jobs, PTYs, favorites,
compatibility, reference scope, safety, and live evidence without overstating
release support.

## Dependencies

- `RAW-FAVORITE-UI-001` — DONE
- `RAW-SEARCH-001` — DONE
- `RAW-RESPONSIVE-001` — DONE
- `RAW-LIVE-001` — DONE
- `RAW-A11Y-001` — DONE
- `RAW-MOUSE-001` — DONE

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

- Run `./scripts/check-docs.sh` and
  `./scripts/verify-raw-mode-evidence.sh`.

## Verification

```bash
./scripts/verify-live-raw-mode.sh
./scripts/verify-raw-mode-evidence.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
