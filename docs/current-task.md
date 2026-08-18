# Current Task

## Task

**ID:** UI-LIVE-COLOR-AUTHORITY-001
**Title:** Keep terminal color output consistent with Yoctui settings
**Status:** IN_PROGRESS

## Objective

Ensure an enabled Yoctui color preference produces the selected terminal
palette even when the parent environment exports `NO_COLOR`, while preserving
the explicit `--no-color` attribute-only mode.

## Dependencies

- `UI-LIVE-RECOVERY-001` — DONE

## Relevant files

- `crates/yoctui-cli/src/main.rs`
- `scripts/test-live-workbench.sh`
- `docs/ui-spec.md`
- `docs/task-registry.toml`
- `docs/implementation-status.md`

## Definition of done

- The terminal backend follows Yoctui's resolved color mode.
- `NO_COLOR=1` cannot silently contradict Color=true in Settings.
- `--no-color` still renders the tested attribute-only palette.
- The live colored PTY gate passes with `NO_COLOR=1` inherited.

## Verification

```bash
NO_COLOR=1 ./scripts/test-live-workbench.sh "$HOME/src/poky/build"
cargo test -p yoctui-ui theme
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
