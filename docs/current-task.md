# Current Task

## Task

**ID:** RAW-PTY-001
**Title:** Execute interactive Raw commands through daemon PTYs
**Status:** IN_PROGRESS

## Objective

Revalidate confirmed interactive Raw intent inside the daemon and route only
catalog-classified native argv through the existing daemon-owned PTY/session
architecture with resize, detach/reattach, writer control, and termination.

## Dependencies

- `RAW-EXEC-MODEL-001` — DONE
- `RAW-CAP-PROBE-001` — DONE
- `PTY-MULTI-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-protocol/src/daemon.rs`
- `crates/yoctui-bitbake/src/raw_job.rs`
- `crates/yoctui-app/src/pty_context.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-cli/src/daemon_pty.rs`
- `crates/yoctui-cli/src/client_runtime.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/architecture.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The daemon accepts only a current confirmed request whose reconstructed
  catalog entry is explicitly `InteractivePty`, with exact current capability,
  executable, build-directory, safety, and preview-digest authority.
- The PTY start path receives an immutable typed command identity and native
  argv; it cannot accept client-supplied executable authority, a joined command,
  or a shell-evaluation string.
- A stable Raw session/request identity maps without collision to one daemon
  PTY session and one Raw execution replica while generic PTY identities cannot
  be decoded as Raw identities.
- Existing PTY process-group ownership, emulator bounds, resize, writer lease,
  input, detach/reattach, termination, exit, and restart-loss behavior remain
  authoritative and publish ordered Raw plus PTY snapshots.
- Client EOF or closing the Raw execution view detaches only that client and
  does not terminate the interactive process; explicit typed termination is
  required.
- Noninteractive catalog entries cannot enter the PTY path, interactive entries
  cannot enter the line-oriented job path, and duplicate/stale/tampered starts
  fail before spawn.
- Model, app, and CLI composition tests cover exact native argv/cwd, input and
  resize, writer transfer, detach/reattach/reconnect, explicit termination,
  normal exit/loss, identity mismatch, stale authority, and zero-spawn denial.

## Verification

```bash
cargo test -p yoctui-model raw_pty
cargo test -p yoctui-app raw_pty
cargo test -p yoctui -- raw_pty
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
