# Current Task

**ID:** SAVED-ENV-DAEMON-001
**Title:** Honor exact profiles in independent diagnosable daemon startup
**Status:** IN_PROGRESS

Dependency ROOTFS-FILES-BROWSER-001 is DONE. Relevant files: CLI daemon_commands,
startup helper and focused fake-profile/process tests. Extract a quiet reusable
startup helper that honors explicit profiles over inherited BUILDDIR, detaches
the child session and records private diagnostics. No daemon is stopped here.
Done means explicit/invalid/fake profile and independent-session tests pass,
formatting/Clippy/roadmap checks pass, docs are updated and change is committed.
Next: SAVED-ENV-LOAD-001. Full suite remains deferred by user instruction.

```bash
cargo test -p yoctui --bin yoctui daemon_start
cargo test -p yoctui --bin yoctui daemon_build_directory
cargo fmt --all --check
cargo clippy -p yoctui --bin yoctui --all-features -- -D warnings
./scripts/verify-roadmap.sh
```

After both tasks and release delivery, restore the external M67 blocker.
