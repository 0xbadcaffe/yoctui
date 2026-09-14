# Current Task

**ID:** REF09-VERIFY
**Title:** Verify and deliver the shared utility refactor
**Status:** IN_PROGRESS

REF09-UTILS is pushed at 301b3d6 (v0.1.100); hosted CI run 34807263171 passes.
REF09-MODULES (v0.1.101) passes 1608 Rust tests, four existing ignored tests,
52 Python bridge tests, strict Clippy, fmt, roadmap/version/CI/layout checks,
and all moved source contracts. All 21 goldens differ only in version digits;
sixteen PNGs reproduce exactly. All 762 reducer arms preserve original tokens,
and all original extracted test functions remain. Every lib.rs is below 1000 lines.

Final work: check documentation, the package graph and final source version;
run the baseline without golden-update flags; inspect hosted CI; record the
handoff and push. Keep the original master checkout's UI edits and four
OpenBMC captures untouched. Historical performance evidence remains historical.
Do not claim fresh live-BitBake performance from fixture tests or moved source.

Verification: cargo fmt --all --check; cargo test --workspace --all-features;
cargo clippy --workspace --all-targets --all-features -- -D warnings;
python3 -m pytest bridge/tests; ./scripts/check-docs.sh;
./scripts/verify-roadmap.sh; python3 scripts/check-version-bump.py;
python3 scripts/check-library-layout.py; locked package archive checks.

All Cargo commands run sequentially with CARGO_BUILD_JOBS=1,
CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0,
and RUST_TEST_THREADS=2. Do not run Cargo during acceptance measurements.
