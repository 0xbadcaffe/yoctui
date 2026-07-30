# Current task

## Active task

**ID:** QA-REPORT-ADAPTER-001
**Title:** Acquire bounded QA findings and reports

## Objective

Acquire exact QA report and log identities supplied by managed jobs or explicit
imports, and normalize bounded typed kernel, URI, patch, license, and
recipe/package findings without inferring evidence from arbitrary filesystem
contents.

## Required work

1. Inspect the typed QA report/finding model, existing security and test-result
   report adapters, filesystem safety helpers, fake-filesystem patterns, and
   the authoritative QA architecture before writing code.
2. Add a focused BitBake report adapter that accepts an exact build identity,
   report generation, scope/check identity, and explicit report candidates
   supplied by a completed managed job or import.
3. Canonicalize and confine every root and report identity, reject symlinks,
   escapes, stale replacements, unsupported file kinds, and duplicate inputs,
   and revalidate identities immediately before reading or opening.
4. Bound candidate count, traversal, file size, total bytes, parsed records,
   fields, and retained text; recognize only documented report formats and
   never scan unrelated build output heuristically.
5. Normalize exact typed finding status, severity, message, task/test/source,
   rule, suggestion, metadata, and report identity while preserving absent
   fields honestly.
6. Preserve valid empty, partial, malformed, missing, permission, timeout,
   cancellation, and worker-loss outcomes distinctly; one bad optional report
   must not discard usable evidence from other exact candidates.
7. Add fake-filesystem and cancellation tests for every normal and relevant
   failure path plus mechanical app response mapping if a new adapter response
   type is required. Do not implement layer execution, CLI polling, or final
   rendering in this task.

## Definition of done

- Only exact supplied and revalidated report identities are read.
- Typed findings retain exact generation, scope, check, report, and source
  correlation within documented hard bounds.
- Empty, partial, malformed, timeout, cancellation, and loss states remain
  distinguishable.
- Focused adapter/app and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-bitbake qa_report
cargo test -p yoctui-app qa_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
