# Current task

## Active task

**ID:** DEVTOOL-JOB-RUNNER-001
**Title:** Add cancellable Devtool process streaming

## Objective

Execute one validated Devtool command asynchronously with bounded streamed
stdout/stderr and deterministic cancellation/terminal events.

## Required work

1. Inventory the ProcessBackend child/process-group, stream framing,
   cancellation timeout/escalation, invalid UTF-8, and fake-process tests.
2. Add a Devtool runner in `yoctui-bitbake` that accepts only a validated
   `DevtoolCommandSpec` plus build directory.
3. Spawn without a shell, pipe both streams, preserve stream identity, bound
   oversized lines, and expose typed started/output/completed/failure events.
4. Distinguish missing executable, spawn failure, nonzero exit, cancellation
   acknowledgement, forced termination after timeout, and unexpected process
   or event-channel loss.
5. Use a Unix process group where available so cancellation covers descendants.
6. Reject a second start while a child is active and make duplicate/late
   cancellation inert.
7. Keep retained job history/state out of this adapter task; the next child
   maps runner events into the existing background-job reducer.
8. Add fake-process adapter tests named `devtool_job_runner` for output,
   invalid UTF-8, truncation, duplicate start, failures, and cancellation.
9. Update architecture documentation for the process/event boundary.

## Definition of done

- Validated typed commands execute without a shell.
- stdout/stderr stream as bounded typed events.
- Every process and cancellation terminal outcome is distinct and tested.
- No terminal suspension or model/UI mutation occurs in the adapter.
- Focused and baseline verification pass.
- Registry/status documents are updated and lifecycle integration is active.

## Verification

```bash
cargo test -p yoctui-bitbake devtool_job_runner
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-JOB-LIFECYCLE-001 — Integrate Devtool persistent job lifecycle`
