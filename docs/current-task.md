# Current Task

## Task

**ID:** VISUAL-TEST-003
**Title:** Add style invariant tests
**Status:** IN_PROGRESS

## Objective

Add aggregate buffer-level tests that enforce the visual semantics shared by
all next-generation workbench scenes, independent of any one golden fixture.

## Dependencies

- `VISUAL-TEST-002` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Exactly one pane exposes the focused-border treatment in each supported
  responsive state.
- Every rendered pane has a non-empty semantic section title and focused panes
  never use the inactive border style.
- Status text maps through semantic theme roles and never depends on color
  alone.
- Task progress presentation agrees with queued, active, terminal, and unknown
  task states.
- Disabled actions remain textual/discoverable and never use enabled or
  selected presentation.

## Verification

```bash
cargo test -p yoctui-ui style_invariants
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
