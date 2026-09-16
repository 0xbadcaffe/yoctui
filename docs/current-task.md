# Current Task

**ID:** M68-CANCEL-001
**Title:** Keep slow operations and build cancellation off the input path
**Status:** IN_PROGRESS

Dependencies: M68-CLONE-001 (DONE).

Scope and done criteria: CLI client/backend operation ownership and named activity; delayed acknowledgement responsiveness and duplicate cancellation tests. Update UI/architecture where changed, registry, status and current task; baseline checks and one coherent commit required.

Verification: `cargo test --workspace --all-features cancel` plus AGENTS.md baseline.
