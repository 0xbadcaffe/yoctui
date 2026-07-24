# Current task

## Active task

**ID:** BB-001
**Title:** Add the real BitBake smoke harness

## Objective

Create and run a live, explicitly configured smoke harness that validates Yoctui against a real initialized Yocto/BitBake environment without treating mocked tests as compatibility evidence.

## Required work

1. Inspect the compatibility document, existing bridge/process adapters, CLI smoke scripts, and any available initialized build directories.
2. Add `scripts/verify-live-bitbake.sh` with explicit environment inputs and safe preflight diagnostics.
3. Exercise real workspace inspection, variable lookup, recipe listing, layer listing, build start, parse/task/log events, normal completion, cancellation, and bridge shutdown.
4. Record the exact BitBake/Yocto versions and backend used only after successful live validation.
5. Keep the live target configurable; use a documented small target by default only when the caller explicitly enables the live test.
6. Never substitute mocked modules, the process fixture backend, or synthetic events for the live matrix.
7. Add deterministic harness self-tests for configuration and failure reporting where they do not claim compatibility.
8. If no usable initialized environment exists, mark this task `BLOCKED` with exact discovery commands and reproduction details, then continue with the next eligible task.

## Definition of done

- The live harness rejects missing or uninitialized build directories clearly.
- A real bridge-backed workspace, metadata queries, build, events, completion, cancellation, and shutdown are exercised.
- Compatibility documentation records only actually observed versions.
- Harness self-tests and the configured live run pass.
- Mock-only evidence is never presented as live support.
- Registry and human-readable status are updated after verification or an exact external blocker is recorded.

## Verification

```bash
./scripts/verify-live-bitbake.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`UI-RESP-001 — Complete the responsive shell matrix`
