# Current Task

## Task

**ID:** UI-VISION-NAV-001
**Title:** Render the grouped IDE-style Navigator
**Status:** IN_PROGRESS

## Objective

Render the stable screen destinations under non-selectable visual group rows
with IDE-style hierarchy, semantic amber accents, and full-row selection while
preserving the existing bounded keyboard and mouse navigation behavior.

## Dependencies

- `UI-VISION-SHELL-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Every stable destination appears once under its specified visual group.
- Group headings are not selectable and do not alter backend state.
- Selected destinations use full-row selection with accessible no-color fallback.
- Existing keyboard and mouse navigation semantics remain bounded.

## Verification

```bash
cargo test -p yoctui-model navigator
cargo test -p yoctui-app navigator
cargo test -p yoctui-ui workbench_navigator
./scripts/verify-roadmap.sh
```
