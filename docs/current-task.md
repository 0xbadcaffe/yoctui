# Current Task

## Task

**ID:** HARDEN-SANITIZER-001
**Title:** Add sanitizer verification

## Objective

Add a reproducible Linux x86_64 nightly sanitizer gate that runs deterministic
native Yoctui workloads under AddressSanitizer and LeakSanitizer.

## Required work

1. Add `scripts/test-sanitizers.sh` with explicit Linux x86_64, nightly, and
   nightly `rust-src` prerequisite checks that fail with actionable exit status
   2.
2. Build the standard library and selected workspace crates with sanitizer
   instrumentation in isolated target directories; do not contaminate normal
   build artifacts.
3. Run the deterministic model and protocol stress targets under
   AddressSanitizer and LeakSanitizer.
4. Run a production CLI headless bridge workload under AddressSanitizer so the
   executable boundary, protocol framing, bridge lifecycle, and shutdown are
   exercised together.
5. Treat every sanitizer finding as a failure. Document unsupported platforms,
   runtime constraints, and exact reproduction commands.

## Definition of done

- AddressSanitizer passes the selected stress tests and headless CLI workload.
- LeakSanitizer passes the selected deterministic stress tests.
- Missing prerequisites and unsupported hosts fail explicitly.
- Focused and baseline verification pass.

## Verification

```bash
./scripts/test-sanitizers.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Document sanitizer scope and prerequisites in `docs/testing.md`.
- Mark `HARDEN-SANITIZER-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `HARDEN-ANALYSIS-001`.

## Next task

`HARDEN-ANALYSIS-001`
