# Current Task

## Task

**ID:** RELEASE-PUBLISH-001
**Title:** Publish verified public packages to crates.io
**Status:** IN_PROGRESS

## Dependencies

- RELEASE-INSTALL-001 — DONE
- LOG-CONSOLE-IMAGE-001 — DONE

## Release candidate

v0.1.64: explicitly approved bounded upstream-cache policy, compatible lru
security fix, source-bound performance evidence and version-only UI fixtures.
All focused M52 checks passed, including fresh real-Poky saturation evidence.
[Validation report](../artifacts/release-quality/cratesio/0.1.64-validation.md).
Historical results do not substitute for newly captured release evidence.

## Next actions

1. Completed: committed candidate and full `./scripts/verify-completion.sh`
   passed from a clean checkout. No unrelated gate was waived.
2. Publish the six public crates in dependency order only after completion and
   package checks pass. Package isolation and native batch dry run passed;
   no upload has occurred.
3. Verify each registry version and a fresh registry installation, record the
   receipt and push the release changes. Do not restart the user's daemon.

```bash
./scripts/verify-completion.sh
./scripts/verify-cratesio-package.sh
cargo owner --list yoctui
```

Use `target` for ordinary release/benchmark caches and `target/ui-performance`
for frame-pointer profiling. These cache settings do not change any acceptance
threshold. Never print registry credentials or publish private e2e/shell crates.
