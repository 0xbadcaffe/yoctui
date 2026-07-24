# Current task

## Active task

**ID:** JOB-001
**Title:** Add the persistent background-job domain model

## Objective

Add the pure, typed model state required for builds and future QEMU, Wic, SDK, testing, Devtool, and maintenance operations to persist independently of workspace navigation.

## Required work

1. Inspect the existing build state and reducer before introducing new types.
2. Add stable job identifiers, kinds, lifecycle states, optional context, progress, bounded output, result/error, timestamps, and cancellation capability in `yoctui-model`.
3. Store jobs independently of the active workspace and keep their history bounded.
4. Add typed reducer actions for queueing, starting, progress, cancellation request, success, failure, cancellation, and loss.
5. Reject or ignore invalid lifecycle transitions deterministically.
6. Add model tests for normal completion, failure, cancellation, invalid transitions, bounded retention, and workspace navigation while a job runs.
7. Do not add process execution or backend cancellation in this task; that belongs to `JOB-002`.

## Definition of done

- Background jobs have a reusable typed domain model with a documented lifecycle.
- Job state survives workspace changes.
- Retention is bounded and observable.
- Invalid transitions cannot corrupt terminal job state.
- Normal, failure, cancellation, loss, and retention paths are covered by reducer tests.
- No process or UI concerns are added to `yoctui-model`.
- Registry and human-readable status are updated after verification.

## Verification

```bash
cargo test -p yoctui-model background_job
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`JOB-002 — Add background-job effect execution and cancellation`
