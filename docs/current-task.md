# Current Task

## Task

**ID:** AUTH-ATTACH-001
**Title:** Harden attached-client startup authority
**Status:** DONE

## Objective

An attached Yoctui client must use the daemon snapshot as its only startup
BitBake/workspace authority, preserve canonical workspace identity without a
retained Workspace build event, and report a valid fallback SSH client IP.

## Dependencies

- STATE-COHERENCE-001 — DONE

## Definition of done

- Attached startup does not select or inspect a client-local metadata backend.
- Top-level daemon workspace identity restores canonical source/build paths.
- Invalid `SSH_CONNECTION` falls through to valid `SSH_CLIENT` data.
- A real client launched outside the initialized shell remains Connected and
  Idle with zero errors.
- Version 0.1.6 is installed and repository completion gates pass.

## Verification

```bash
cargo test -p yoctui-app daemon_attach_uses_top_level_workspace_identity
cargo test -p yoctui-protocol daemon_workspace_event_updates_persistent_workspace_identity
cargo test -p yoctui-protocol daemon_recovery_keeps_current_workspace_over_stale_persisted_identity
cargo test -p yoctui attached_startup_uses_only_daemon_metadata
cargo test -p yoctui ssh_access_origin_falls_back_to_valid_client_variable
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

The installed 0.1.6 release was additionally captured from a shell without the
Poky environment. It attached to the release daemon as Project `poky`, MACHINE
`qemux86-64`, DISTRO `poky`, release `6.0.2`, with Active 0, Waiting 0, Errors
0, and no client-local backend failure.
