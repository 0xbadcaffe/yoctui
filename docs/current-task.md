# Current Task

## Task

**ID:** TASKS-UI-002
**Title:** Improve overall build progress presentation
**Status:** IN_PROGRESS

## Objective

Add a compact Tasks workspace build summary and strong overall progress bar
using only authoritative or honestly derived aggregate values.

## Dependencies

- `TASKS-UI-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Completed/total, active, waiting, warnings, errors, and elapsed use typed
  build/task state.
- Sstate reuse appears only if an authoritative value exists.
- The overall bar is determinate only when total tasks are known and nonzero.
- Unknown totals remain explicitly indeterminate without a fake percentage.
- The summary remains compact and useful at supported widths.

## Verification

```bash
cargo test -p yoctui-ui next_generation_build_summary
cargo test -p yoctui-model build_summary
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
