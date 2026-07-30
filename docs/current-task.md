# Current task

## Active task

**ID:** TEST-RUNNER-ADAPTER-001
**Title:** Adapt typed Yocto test execution

## Objective

Add safe adapters for selftest capability discovery, exact shell-free command
construction, independently polled execution, bounded typed output, and
graceful/forced cancellation.

## Required work

1. Add a focused `yoctui-bitbake` Testing adapter that discovers and
   canonicalizes `oe-selftest` and `bitbake-selftest` from the initialized
   environment without guessing paths.
2. Revalidate executable identity at launch and construct the exact indexed
   shell-free argv for each typed `TestSelftestRequest`.
3. Apply `BB_SKIP_NETTESTS=yes` only to the BitBake-selftest child environment;
   never mutate the Yoctui process environment.
4. Reuse managed `BuildRequest` execution for testimage, testsdk, testsdkext,
   and configured ptest rather than duplicating BitBake process ownership.
5. Add one process-group-owned runner with bounded stdout/stderr events,
   duplicate-start rejection, success, nonzero failure, timeout, worker loss,
   and graceful cancellation with forced escalation.
6. Add mechanical app event mapping and fake-filesystem/fake-process tests for
   exact commands, tampering, bounds, cancellation, and every terminal path.

## Definition of done

- Capability states distinguish available, missing, and failed discovery with
  canonical executable identity.
- Exact commands are reconstructable from typed requests without shell strings.
- Runner events retain stream identity, truncation, and distinct terminal
  meaning.
- Cancellation owns the process group, attempts graceful termination, and
  reports forced escalation honestly.
- Fake-process tests cover normal, invalid, duplicate, nonzero, timeout,
  cancellation, and lost-worker paths without claiming live compatibility.

## Verification

```bash
cargo test -p yoctui-bitbake test_runner
cargo test -p yoctui-app test_runner
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
