# Current Task

## Task

**ID:** UI-LITERAL-COCKPIT-001
**Title:** Match the reference Tasks cockpit and Inspector
**Status:** IN_PROGRESS

## Objective

Match the reference's task table, selected-task log, job history, metadata,
recent log, actions, and system-status composition at `160x48`.

## Dependencies

- `UI-LITERAL-NAV-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-ui/tests/golden/literal-reference-160x48.cells`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Canonical center tiers are exactly 17, 18, and 9 rows.
- Canonical Inspector sections are exactly 16, 15, 7, and 6 rows.
- Task rows use compact reference labels and status symbols.
- Selected task metadata and log path remain authoritative and readable.
- Job history uses retained typed jobs/build records without illustrative data.

## Verification

```bash
cargo test -p yoctui-ui literal_cockpit
cargo test -p yoctui-ui workbench_tasks
cargo test -p yoctui-model background_job
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
