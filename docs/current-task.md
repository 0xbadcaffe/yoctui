# Current Task

**ID:** BACKEND-PROBE-RECOVERY-001
**Title:** Retry inconclusive backend authority without repeating tool discovery
**Status:** IN_PROGRESS

User screenshot overrides the blocked queue. Preserve positive evidence and
retry only inconclusive backend API discovery with exact identity checks and
bounded background attempts. Publish a new capability generation and restart
inventory after recovery. Test failures, success, unsupported APIs and cancellation.
Update specification, architecture, registry and status; commit.

```bash
cargo test -p yoctui --bin yoctui backend_recovery
cargo test -p yoctui-bitbake backend_probe
cargo fmt --all --check
./scripts/verify-roadmap.sh
```

Next: BACKEND-PROBE-RECOVERY-RELEASE-001, v0.1.242 with live fresh-client Kernel
inspection. Full workspace tests remain deferred. Existing provider and M67
external blockers remain recorded.
