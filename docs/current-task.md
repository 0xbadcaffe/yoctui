# Current Task

## Task

**ID:** THEME-PACKRAT-004
**Title:** Update source-preview palette assertions
**Status:** DONE

## Objective

Final completed task: align source-preview TestBackend assertions with the Dark
Pro Packrat palette.

## Verification

```bash
cargo test -p yoctui-ui bitbake_preview_highlights_assignments_and_comments
cargo test --workspace --all-features
```

## Definition of done

- Source-preview semantic colors match Dark Pro exactly.

## Next task

## Terminal handoff

All registry tasks are complete.
