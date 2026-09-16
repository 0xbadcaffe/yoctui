# Current Task

**ID:** M68-GITUI-001
**Title:** Integrate GitUI through the existing terminal workbench
**Status:** IN_PROGRESS

Dependencies: M68-GIT-001 (DONE).

Scope and done criteria: typed menu/palette launch, terminal runtime, availability diagnostic; real temporary-repository terminal smoke plus input/resize tests. Update UI/architecture where changed, registry, status and current task; baseline checks and one coherent commit required.

Verification: `cargo test --workspace --all-features gitui` plus AGENTS.md baseline.
