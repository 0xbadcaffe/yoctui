# Current Task

## Task

**ID:** RAW-001
**Title:** Complete Raw BitBake Command Workbench
**Status:** IN_PROGRESS

## Objective

Run the complete Raw workbench completion gate after all required model, UI,
security, compatibility, live, and documentation tasks are complete.

## Dependencies

- `RAW-FAVORITE-UI-001` — DONE
- `RAW-SEARCH-001` — DONE
- `RAW-RESPONSIVE-001` — DONE
- `RAW-DOC-001` — DONE
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

- Run `./scripts/verify-completion.sh` and
  `./scripts/verify-roadmap.sh`.

## Verification

```bash
./scripts/verify-completion.sh
./scripts/verify-roadmap.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
