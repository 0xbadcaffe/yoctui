# Current task

## Active task

**ID:** JOB-002
**Title:** Add background-job effect execution and cancellation

## Objective

Connect the existing asynchronous build execution path to the shared background-job model so build lifecycle, output, completion, failure, and cancellation remain observable while the user navigates other workspaces.

## Required work

1. Inspect the existing CLI build loop, `yoctui-app` effect boundary, and backend cancellation adapters before changing them.
2. Allocate a stable background-job ID when a confirmed build is started.
3. Drive the job through queued, starting, running, progress/output, and terminal states from typed backend events.
4. Associate the current target and Tasks workspace context with the job.
5. Keep event handling and cancellation active while the user changes workspaces.
6. Map cancellation request, acknowledgement, backend failure, and disconnect/loss without reporting false success.
7. Add app orchestration tests and fake backend/process cancellation tests for success, failure, cancellation, and navigation persistence.
8. Preserve the existing typed backend boundary and do not parse raw process text in UI code.

## Definition of done

- A confirmed build creates exactly one background job.
- Typed backend events update that job without widget parsing.
- The job remains active and inspectable across workspace changes.
- Normal completion, build failure, cancellation acknowledgement, cancellation failure, and backend loss produce correct terminal states.
- Existing build controls continue to work.
- Fake integration coverage passes without claiming live BitBake compatibility.
- Registry and human-readable status are updated after verification.

## Verification

```bash
cargo test -p yoctui-app background_job
cargo test -p yoctui-bitbake cancellation
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`BB-001 — Add the real BitBake smoke harness`
