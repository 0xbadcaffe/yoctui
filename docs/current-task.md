# Current Task

**ID:** REF09-UTILS
**Title:** Consolidate portable utilities and repair shared helper bugs
**Status:** IN_PROGRESS

User override: review recent commits and tasks, share common code through
`yoctui-utils`, split oversized library entry points, remove machine-specific
runtime defaults, fix bugs, bump versions and push verified commits.

The prior 715 tasks are DONE. REF09-UTILS precedes REF09-MODULES and
REF09-VERIFY. Preserve the original checkout's UI edits and four untracked
OpenBMC captures; implementation runs on branch `ref09` in a separate worktree.
Historical evidence paths identify actual measurements and must not be relabeled.

Definition of done: shared helpers have focused failure-path tests, all relevant
crates consume the helpers, published dependencies include utils and exact
workspace versions, no machine-specific runtime identity is required.

Verification: `cargo test -p yoctui-utils`; `cargo test --workspace --all-features`;
`cargo clippy --workspace --all-targets --all-features -- -D warnings`;
`cargo fmt --all --check`; `python3 -m pytest bridge/tests`;
`./scripts/verify-roadmap.sh`; `python3 scripts/check-version-bump.py`.
Run Cargo sequentially with CARGO_BUILD_JOBS=1, CARGO_PROFILE_DEV_DEBUG=0,
CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0 and RUST_TEST_THREADS=2.

No live build or performance measurement is authorized by this refactor.
Retained acceptance evidence remains historical; changed runtime sources require
fresh measurement before any new live performance claim.
