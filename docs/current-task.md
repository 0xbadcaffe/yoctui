# Current task

## Active task

**ID:** SEC-REPORT-ADAPTER-001
**Title:** Acquire and parse Security reports

## Objective

Acquire only explicit canonical Security report paths and parse supported CVE
and SPDX content into bounded typed records.

## Required work

1. Add report acquisition to the Security adapter without walking outside
   explicit request paths.
2. Accept canonical regular non-symlink files or bounded canonical
   directories; reject relative paths, root, escape, stale, duplicate, and
   unsupported entries.
3. Bound traversed directories, entries, files, per-file/total bytes, records,
   fields, and elapsed time; expose cancellation and worker loss distinctly.
4. Fingerprint exact file identities before parsing and preserve them in every
   typed record.
5. Parse supported Yocto CVE JSON/text into typed ID, recipe/package,
   product/version, status, severity/score/vector/link/summary, mappings,
   metadata, and limitations.
6. Parse supported SPDX JSON summaries and components while retaining
   archives/unsupported schemas as exact artifacts with limitations.
7. Preserve valid records beside malformed/oversized inputs as partial and
   distinguish valid empty from total failure.
8. Add fake-filesystem tests for normal, empty, mixed partial, malformed,
   oversized, symlink, escape, timeout, cancellation, and loss paths.

## Definition of done

- Acquisition is explicit, canonical, bounded, cancellable, and fail-closed.
- Supported CVE and SPDX data becomes only typed model records.
- Exact identities survive parsing and stale/unsafe paths are rejected.
- Empty, partial, malformed, oversized, timeout, cancellation, and loss remain
  distinct.
- Focused report and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake security_report
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
