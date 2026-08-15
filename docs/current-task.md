# Current Task

## Task

**ID:** DAEMON-ATTACH-BUILD-001
**Title:** Restore live build progress after daemon attach
**Status:** DONE

## Objective

Make daemon-owned builds publish and retain typed workspace, build, parse,
runqueue, task, log, and terminal state so a newly attached client immediately
renders the authoritative live build cockpit and continues updating it through
the normal model reducer.

The fix must preserve daemon ownership across detach, bounded protocol state,
unknown-progress honesty, presentation state, and typed UI boundaries. It must
cover fresh snapshot restoration, ordered incremental task progress, terminal
outcomes, and failure/disconnect behavior before reinstalling Yoctui and
rerunning `core-image-minimal` in the configured Poky build directory.

The optimized installed binary is running daemon instance
`a68edcb8ebf4694191776e2a2fde3256`. A fresh release-client attach restored the
typed bridge workspace as Poky 5.0.19, showed `BB Running`, target
`core-image-minimal`, status `Parsing`, and authoritative task progress
`94/4090`; detaching left the daemon-owned build running. The previous stale
setup notice and unsafe `/` fallback are absent.

## Dependencies

- `DAEMON-001` — DONE
- `BRIDGE-PROGRESS-001` — DONE
- `TELEMETRY-COCKPIT-001` — DONE

## Relevant files

- `crates/yoctui-protocol/src/daemon.rs`
- `crates/yoctui-cli/src/daemon_bitbake.rs`
- `crates/yoctui-cli/src/client_runtime.rs`
- `crates/yoctui-cli/src/main.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/product-roadmap.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Daemon build workers publish every supported typed build lifecycle event.
- The bounded daemon snapshot retains enough typed build state for fresh attach.
- Attach installs build/task state without changing client presentation state.
- Ordered incremental task totals, lifecycle, and percentages reach existing
  Dashboard and Tasks progress meters.
- Terminal success, failure, cancellation, and backend loss remain distinct.
- Fake-backend and replica tests cover initial attach and incremental updates.
- Focused and baseline checks pass.
- The release binary is installed and a real daemon-owned Poky build is started;
  status/attach evidence shows non-placeholder build progress.
- Registry/status/current-task documentation is updated and committed.

All definition-of-done items and verification commands pass. This final
completed task remains the terminal handoff because every registry task is
`DONE`.

## Verification

```bash
cargo test -p yoctui daemon_attach_build
cargo test -p yoctui-app daemon_attach_build
cargo test -p yoctui-protocol daemon_build
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
