# Current task

## Active task

**ID:** SEC-ADAPTER-001
**Title:** Adapt CVE and SPDX metadata and reports

## Objective

Implement bounded Security capability inspection, CVE/SPDX report acquisition
and parsing, and the exact package-mapping runner.

## Required work

1. Inspect the typed Security model and existing bounded filesystem/process
   adapters before adding a new adapter module.
2. Build capability snapshots only from explicit build/recipe/image metadata,
   canonical report roots, and canonical PATH tool discovery.
3. Preserve exact authoritative task names including current
   `create_recipe_sbom` and legacy `create_spdx` without release-name guessing.
4. Acquire only explicit request paths, reject symlinks/escapes/stale identity,
   and bound directories, files, bytes, records, fields, and elapsed time.
5. Parse supported CVE JSON/text and SPDX JSON into typed records; keep
   unknown status/schema, empty, partial, malformed, and oversized states
   explicit.
6. Reconstruct and revalidate the exact shell-free package-mapping operation;
   emit bounded typed stream and terminal events with cancellation/timeout.
7. Add fake filesystem/process tests for normal and every relevant failure
   path. Do not claim live compatibility.

## Definition of done

- Capability data is authoritative, canonical, and fail-closed.
- CVE/SPDX parsing returns only bounded typed records and limitations.
- Package mapping is shell-free and immediately revalidated.
- Empty, partial, malformed, timeout, cancellation, nonzero, rejection, and
  loss remain distinct.
- Focused adapter/app and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake security
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
