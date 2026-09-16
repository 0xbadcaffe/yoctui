# Current Task

**ID:** M68-CLONE-001
**Title:** Run fresh clones asynchronously with named Braille progress
**Status:** NOT_STARTED

User M68 request supersedes the blocked M67 live-evidence follow-up.
Dependencies: none. Scope: environment adapter, typed pending operations,
CLI clone worker, Braille progress rendering. Done: reviewed fresh destinations
work, input/redraw stay responsive, Cloning… is visible until a typed success,
failure or cancellation, duplicate launches are prevented, tests cover all paths.
Verify: `cargo test --workspace --all-features clone` plus AGENTS.md baseline.
Update UI/architecture, status and registry; commit, then M68-CANCEL-001.
