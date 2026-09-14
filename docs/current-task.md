# Current Task

**ID:** CONCEPT-VERIFY
**Title:** Verify six production concept images and deliver layout changes
**Status:** IN_PROGRESS

CONCEPT-SHELL and CONCEPT-DETAIL implement the six scene layouts through the
production render_at path. Checkpoint v0.1.102 tags 3edb883. The shell is pushed
at 75e9d44; detail layout is v0.1.105. 202 app and 285 UI tests pass without
golden updates, plus 52 bridge tests and the concept/raster corruption checks.

Finish the full workspace baseline (cargo test --workspace --all-features,
strict Clippy, formatting), documentation/CLI checks, version/layout/roadmap,
and deterministic raster checks. Review all six PNGs and the review document
at docs/design/concept-layout-review.md, then commit and push the final version.

Original concept PNGs and historical live captures remain unchanged. These
layout captures are production-renderer fixtures; no fresh live-Poky performance
certification is claimed. The original master checkout's user edits remain
untouched. Cargo runs sequentially with one build job and no debug/incremental
artifacts; RUST_TEST_THREADS=2 bounds runtime tests.
