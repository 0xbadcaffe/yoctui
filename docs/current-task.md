# Current Task

**ID:** REF09-MODULES
**Title:** Split oversized library entry points by responsibility
**Status:** IN_PROGRESS

User override: review recent commits and tasks, share common code through
`yoctui-utils`, split oversized library entry points, remove machine-specific
runtime defaults, fix bugs, bump versions and push verified commits.

The prior 715 tasks are DONE. REF09-UTILS precedes REF09-MODULES and
REF09-VERIFY. Preserve the original checkout's UI edits and four untracked
OpenBMC captures; implementation runs on branch `ref09` in a separate worktree.
Historical evidence paths identify actual measurements and must not be relabeled.

REF09-UTILS is complete in v0.1.100: 1608 Rust tests pass (four existing ignored),
52 Python tests pass, strict Clippy, fmt, version, roadmap and both raster
galleries pass. All eight consuming crates use utils; the public package
verification graph now includes it.

Definition of done: every workspace lib.rs is at most 1000 lines, public APIs
and all existing tests remain available, exact-test scripts follow moved
modules, and source contracts inspect the actual implementation files.

Verification: `cargo test -p yoctui-utils`; `cargo test --workspace --all-features`;
`cargo clippy --workspace --all-targets --all-features -- -D warnings`;
`cargo fmt --all --check`; `python3 -m pytest bridge/tests`;
`./scripts/verify-roadmap.sh`; `python3 scripts/check-version-bump.py`.
Run Cargo sequentially with CARGO_BUILD_JOBS=1, CARGO_PROFILE_DEV_DEBUG=0,
CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0 and RUST_TEST_THREADS=2.

No live build or performance measurement is authorized by this refactor.
Retained acceptance evidence remains historical; changed runtime sources require
fresh measurement before any new live performance claim.
