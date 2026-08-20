# Current Task

## Task

**ID:** VISUAL-TEST-002
**Title:** Create target-design golden tests
**Status:** IN_PROGRESS

## Objective

Create reviewed canonical TestBackend cell-and-style goldens for the target
design's idle Dashboard, active Tasks build, selected failed task, and daemon
reconnect/degraded scenes. Updates must remain explicit and diff-reviewable.

## Dependencies

- `VISUAL-TEST-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Four reviewed golden buffers cover idle Dashboard, active Tasks build,
  selected failed task, and daemon reconnect/degraded state.
- Every fixture uses typed model data, a fixed clock, and canonical dimensions.
- Goldens serialize terminal symbols and semantic styles and fail with an exact
  cell coordinate when presentation changes.
- Updates require an explicit environment switch/script and produce a
  reviewable fixture diff; ordinary tests never accept changes.

## Verification

```bash
cargo test -p yoctui-ui target_design_golden
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
