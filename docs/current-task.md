# Current Task

## Task

**ID:** RAW-A11Y-001
**Title:** Verify Raw Mode accessibility
**Status:** IN_PROGRESS

## Objective

Preserve focus, selection, availability, safety, favorite, job, and PTY meaning
in every theme, no-color, high-contrast, and reduced-motion mode.

## Dependencies

- `RAW-FAVORITE-UI-001` — DONE
- `RAW-SEARCH-001` — DONE
- `RAW-RESPONSIVE-001` — DONE

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

- Every Raw state retains semantic text and explicit focus without color.
- Dialog and PTY traps remain isolated from global shortcuts.
- TestBackend tests cover themes, no-color, reduced motion, and bounds.

## Verification

```bash
cargo test -p yoctui-ui raw_accessibility
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
