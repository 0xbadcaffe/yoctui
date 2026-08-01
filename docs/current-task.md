# Current Task

## Task

**ID:** HARDEN-FUZZ-001
**Title:** Add reproducible fuzz harnesses

## Objective

Add bounded, reproducible cargo-fuzz coverage for the two principal untrusted
pure-data boundaries: protocol framing and reducer-owned retained state.

## Required work

1. Inspect and reuse the existing protocol decode and model retention APIs;
   do not copy their implementations into fuzz targets.
2. Add a cargo-fuzz package excluded from the normal workspace and one target
   each for arbitrary protocol frames and arbitrary retained-state operations.
3. Keep target input and in-memory work bounded so finite smoke runs are
   deterministic and suitable for CI/manual verification.
4. Add minimal checked-in corpus seeds covering valid, malformed, oversized,
   unknown-version, and retention-pressure inputs.
5. Add `scripts/test-fuzz.sh` with explicit nightly/cargo-fuzz prerequisite
   errors and finite smoke runs for every target.
6. Document the exact fuzz commands, artifact location, and the fact that a
   finite smoke run is not an exhaustive safety claim.

## Definition of done

- Both fuzz targets compile and complete their finite smoke budgets without a
  crash.
- Corpus seeds are bounded and checked in.
- Missing prerequisites fail with actionable exit status 2.
- Focused and baseline verification pass.

## Verification

```bash
./scripts/test-fuzz.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Document fuzzing in `docs/testing.md`.
- Mark `HARDEN-FUZZ-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `HARDEN-STRESS-001`.

## Next task

`HARDEN-STRESS-001`
