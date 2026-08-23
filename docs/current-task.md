# Current Task

## Task

**ID:** RAW-CATEGORY-UI-001
**Title:** Implement Raw category browser
**Status:** IN_PROGRESS

## Objective

Replace the Raw Mode landing placeholder with the first bounded catalog-backed
browser level: Favorites pinned first, followed by the exact reference-derived
category hierarchy and visible category classifications.

## Dependencies

- `RAW-NAV-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The outer Raw Workspace renders Favorites before every reference-derived
  category in catalog order without parsing the Markdown reference at runtime.
- Executable BitBake, reference-only, conceptual, companion-tool, and Favorites
  categories have distinct textual classifications that remain meaningful
  without color.
- Up/Down and `j`/`k` move one bounded category selection; Right/`l` and Enter
  activate the command-browser state without inventing the command-list UI
  owned by `RAW-COMMAND-UI-001`; Left/`h` remains bounded at the outer level.
- Empty, selected, first, last, and long category states render safely with
  explicit bounded viewport position.
- Wide, medium, narrow, and below-minimum layouts preserve category identity,
  focus, and search state and never panic.
- TestBackend tests cover catalog order, classification text, selection,
  scrolling, keyboard routes, accessibility without color, and responsive
  boundaries.

## Verification

```bash
cargo test -p yoctui-ui raw_category
cargo clippy -p yoctui-model -p yoctui-app -p yoctui-ui --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
