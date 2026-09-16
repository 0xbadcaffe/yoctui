# Current Task

**ID:** M68-GIT-001
**Title:** Show asynchronous source repository synchronization status
**Status:** IN_PROGRESS

Dependencies: M68-CANCEL-001 (DONE).

Scope and done criteria: typed model Git status, shell-free CLI adapter and global header; temporary repository tests for dirty, ahead, behind, missing upstream and failures. Update UI/architecture where changed, registry, status and current task; baseline checks and one coherent commit required.

Verification: `cargo test --workspace --all-features git_status` plus AGENTS.md baseline.
