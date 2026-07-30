# Current task

## Active task

**ID:** TEST-RESULT-ADAPTER-001
**Title:** Adapt resulttool import comparison and export

## Objective

Add safe resulttool capability discovery, explicit bounded test-result import,
exact comparison, and non-overwriting JUnit export adapters.

## Required work

1. Discover and canonicalize `resulttool` from the initialized PATH snapshot
   independently of selftest capability.
2. Accept only explicit canonical regular non-symlink `testresults.json` files
   or validated retained directories; bound traversal, files, bytes, parsed
   runs, suites, cases, metadata, logs, and limitations.
3. Parse supported result JSON into typed model records while keeping malformed,
   duplicate, oversized, missing, empty, partial, timeout, cancellation, and
   worker-loss outcomes distinct.
4. Revalidate exact result path, size, timestamp, and fingerprint for
   `regression-file`; construct its indexed shell-free vector and return
   identity-correlated typed comparisons.
5. Revalidate result identity and destination parent immediately before JUnit
   spawn; construct `resulttool junit <json> -j <destination>` and fail closed
   if the destination exists, is a symlink, escapes, or its parent changed.
6. Add mechanical app response mapping and fake filesystem/process tests for
   import bounds, malformed data, exact vectors, tampering, non-overwrite,
   cancellation, timeout, nonzero, and loss.

## Definition of done

- Resulttool capability is independent, canonical, and revalidated.
- Import emits only bounded typed records from explicit validated roots.
- Comparison and export commands are exact, shell-free, identity-correlated,
  and stale-safe.
- JUnit export cannot overwrite an existing or changed destination.
- Fake tests cover normal, empty, partial, malformed, bounded, stale,
  cancellation, timeout, nonzero, and worker-loss paths without claiming live
  compatibility.

## Verification

```bash
cargo test -p yoctui-bitbake test_results
cargo test -p yoctui-app test_results
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
