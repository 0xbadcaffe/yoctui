# Current Task

**ID:** M69-HISTORY-001
**Title:** Persist and browse bounded saved builds offline
**Status:** IN_PROGRESS

Dependencies: M69-AUTHORITY-001 (DONE).

Files and done criteria: protocol archive schema, CLI atomic persistence and startup load, typed History details; restart/corrupt/missing/truncated records and UI tests. Baseline checks, UI/architecture documentation, registry/status/current updates and coherent commit required.

Verification: `cargo test --workspace --all-features archive` plus AGENTS.md baseline.
