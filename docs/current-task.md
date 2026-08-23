# Current Task

## Task

**ID:** RAW-JOB-001
**Title:** Execute noninteractive Raw commands as daemon jobs
**Status:** IN_PROGRESS

## Objective

Revalidate confirmed noninteractive Raw intent inside the daemon, reconstruct
and spawn exact native argv without a shell, and retain the bounded job across
client detach, cancellation, reconnect, and terminal loss.

## Dependencies

- `RAW-EXEC-MODEL-001` — DONE
- `RAW-CAP-PROBE-001` — DONE
- `CLIENT-ARCH-001` — DONE

## Relevant files

- `crates/yoctui-bitbake/src/lib.rs`
- `crates/yoctui-bitbake/src/raw_job.rs`
- `crates/yoctui-protocol/src/daemon.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-cli/src/client_runtime.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/architecture.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- The daemon accepts only a current versioned `StartRaw` request reviewed
  against its exact state generation, normalized built-in catalog, connected
  capability snapshot, capability generation, environment/build identity, and
  catalog-declared noninteractive interaction and safety classes.
- Daemon-side planning reconstructs every template and additional argument as
  native argv, resolves and revalidates the authoritative BitBake executable,
  recomputes the indexed preview digest, and rejects any mismatch before
  process construction; it never accepts a command string or invokes a shell.
- A daemon-owned supervisor allocates non-reused Raw request/job/stream
  identities, owns the child process group, independently bounds stdout and
  stderr with explicit drop/truncation accounting, journals lifecycle
  snapshots, and survives the initiating client connection.
- Normal completion, nonzero exit, spawn rejection, timeout, cancellation
  request/acknowledgement, bounded graceful then forced termination, unexpected
  channel/runner loss, and daemon shutdown/recovery map exactly once to typed
  terminal state without resurrection from delayed output.
- Start/cancel commands return their original request correlation, reject
  stale generation, duplicate work, wrong interaction/owner identity, and
  cancellation of unowned or terminal work, and publish ordered daemon
  sequence/generation snapshots for every attached client.
- Client effects convert the exact confirmed request to `StartRaw`, never start
  a local Raw process while attached, and install current reconnect snapshots;
  detach or client EOF does not cancel daemon work.
- Fake-process, protocol, app, and CLI composition tests cover exact native
  argv/cwd, zero-spawn denial, bounded Unicode output, success/nonzero/spawn
  failure/timeout/loss, graceful and forced cancellation, duplicate/stale
  requests, detach/reattach, reconnect snapshot, and fail-closed tampering.

## Verification

```bash
cargo test -p yoctui-bitbake raw_job
cargo test -p yoctui-protocol raw_job
cargo test -p yoctui-app raw_job
cargo test -p yoctui -- raw_job
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
