# Current task

## Active task

**ID:** SEC-MAPPER-ADAPTER-001
**Title:** Run exact CVE package mapping

## Objective

Reconstruct and run only the capability-inspected `cve-check-map-pkgs`
operation as one bounded, cancellable, shell-free Security runner.

## Required work

1. Add a Security package-mapping command specification and independent runner
   to `yoctui-bitbake`.
2. Construct the vector only from a typed operation preview and immediately
   revalidate the exact canonical regular non-symlink executable and every
   required canonical input identity before spawn.
3. Use native argv with no shell, process-global environment mutation, or
   reconstruction from display text.
4. Bound stdout/stderr lines, retained output, arguments, execution time, and
   cancellation escalation while preserving invalid UTF-8 lossily.
5. Emit typed started, bounded stdout/stderr, success, nonzero failure,
   cancellation requested/rejected/completed, timeout, and worker-loss events
   that preserve the exact Security session ID.
6. Reject duplicate starts, stale/tampered previews, missing or symlinked
   inputs/tools, unsafe arguments, stream loss, and process-control failures.
7. Add fake-process tests for exact argv, success, bounded streams, nonzero
   exit, validation rejection, duplicate start, graceful/forced cancellation,
   timeout, and worker loss.
8. Update app normalization tests only where needed to prove typed events cross
   the boundary without parsing process output.

## Definition of done

- The exact inspected mapper operation is revalidated immediately before a
  shell-free spawn.
- Output and lifecycle events are bounded, typed, session-correlated, and
  independent from the managed BitBake coordinator.
- Success, nonzero, rejection, cancellation, timeout, stream/worker loss, and
  process-control failures remain distinct.
- Focused mapper/app and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake security_mapper
cargo test -p yoctui-app security_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
