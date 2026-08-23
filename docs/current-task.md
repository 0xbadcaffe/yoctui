# Current Task

## Task

**ID:** RAW-LIVE-001
**Title:** Validate representative Raw commands against supported BitBake
**Status:** IN_PROGRESS

## Objective

Run representative read-only, build/task, cancellation, reconnect, and
interactive PTY commands against supported environments with exact capability
and evidence identities.

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

- Representative commands run only under current capability authority.
- Cancellation, reconnect, and PTY lifecycle retain request identity and
  terminal output semantics.
- Live evidence captures exact supported environment identities.

## Verification

```bash
./scripts/verify-live-raw-mode.sh
./scripts/verify-raw-mode-evidence.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
