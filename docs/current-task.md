# Current Task

**ID:** DAEMON-STARTUP-002
**Title:** Serve daemon clients while compatibility discovery runs
**Status:** IN_PROGRESS

Dependency FRESH-CLONE-001 is DONE. Scope: CLI startup worker, compatibility
probe cleanup and fake-process integration tests. Done: IPC startup does not
wait for compatibility; typed authority updates existing clients before recipe
inventory starts; commands retain loading/unknown guards; shutdown cancels and
cleans up discovery. Verify: `cargo test -p yoctui --test daemon_startup` and the
AGENTS.md baseline. Update UI/architecture, task registry and implementation
status; commit and run the completion gate.
