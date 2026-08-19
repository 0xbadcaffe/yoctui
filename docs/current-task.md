# Current Task

## Task

**ID:** JOB-UI-001
**Title:** Redesign Job History
**Status:** IN_PROGRESS

## Objective

Make retained background work readable as a stable responsive table with exact
typed lifecycle distinctions and selection-driven detail.

## Dependencies

- `FOUNDATION-UI-003` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The table has stable status, operation type, target/context, start, finish,
  and elapsed columns with responsive hiding.
- Active jobs remain pinned or otherwise unmistakably visible.
- Failed, cancelled, and lost remain exact distinct terminal states.
- Selection opens or drives authoritative job detail.
- Empty history and unavailable timestamps remain explicit.
- Keyboard and existing mouse selection behavior remain typed.

## Verification

```bash
cargo test -p yoctui-ui next_generation_job_history
cargo test -p yoctui-model background_job
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
