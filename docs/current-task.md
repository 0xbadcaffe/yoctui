# Current Task

## Task

**ID:** JOB-UI-002
**Title:** Add compact job summary
**Status:** IN_PROGRESS

## Objective

Add a compact, reusable summary of retained and active background work using
only counts available from typed model state.

## Dependencies

- `JOB-UI-001` — DONE

## Relevant files

- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- A compact summary shows authoritative active, queued, failed, and recently
  completed counts.
- Daemon-owned work is labeled and counted only where ownership is known.
- Embedded and standalone Job History surfaces share the same projection.
- Compact and wide forms preserve every required state without relying on
  color.
- Empty history remains explicit and no count is fabricated.

## Verification

```bash
cargo test -p yoctui-ui next_generation_job_summary
cargo test -p yoctui-model background_job
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
