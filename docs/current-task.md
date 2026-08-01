# Current Task

## Task

**ID:** HARDEN-ANALYSIS-001
**Title:** Integrate hardening analysis gates

## Objective

Audit and integrate every existing and newly added hardening technique into a
single strict, documented completion path with reproducible artifacts and
actionable prerequisite failures.

## Required work

1. Execute and audit the existing pseudo-terminal, Valgrind, deterministic
   profile, and Flamegraph scripts. Fix correctness or determinism defects;
   never weaken a finding or missing-tool failure.
2. Confirm the model and protocol property tests run in the ordinary workspace
   suite and the real process-tree test runs in the stress gate.
3. Add fuzz, stress/process-tree, sanitizer, and pseudo-terminal gates to
   `scripts/verify-completion.sh` in a clear order before memory/profile output.
4. Add the deterministic stress gate to CI. Keep nightly/tool-heavy fuzz,
   sanitizer, Valgrind, and Flamegraph work in the strict completion gate with
   documented local prerequisites.
5. Verify Valgrind summaries reject definite/indirect leaks and unexpected
   file descriptors, profiling writes a nonempty timing artifact, and
   Flamegraph writes a nonempty SVG.
6. Update testing/profiling documentation and record exact tool/platform
   limitations without claiming live BitBake coverage.

## Definition of done

- Terminal, Valgrind, profile, and Flamegraph scripts pass and produce their
  expected evidence.
- The strict completion script invokes every hardening technique.
- CI runs the portable deterministic stress gate.
- Focused and baseline verification pass.

## Verification

```bash
./scripts/test-terminal.sh
./scripts/valgrind.sh
./scripts/profile-workload.sh
./scripts/flamegraph.sh
test -s artifacts/valgrind/report.xml
test -s artifacts/valgrind/summary.txt
test -s artifacts/profile/summary.txt
test -s artifacts/flamegraph/yoctui.svg
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/testing.md` and `docs/profiling.md` for the integrated gate.
- Mark `HARDEN-ANALYSIS-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `HARDEN-001`.

## Next task

`HARDEN-001`
