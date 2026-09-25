# Current Task

**ID:** DEVTOOL-WORKSPACE-RELEASE-001
**Title:** Package and install the Devtool Workspace feature series
**Status:** IN_PROGRESS

Bump the workspace release to v0.1.229, update version-bearing artifacts, run
the focused Devtool Workspace and patch checks plus strict workspace Clippy,
build and install the release binary, stop stale Yoctui daemons, start one fresh
daemon from the initialized Romulus build environment, commit, and push. Keep
the user's existing untracked capture artifacts untouched. The full workspace
test suite remains deferred until the user requests it.

Verify with:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
