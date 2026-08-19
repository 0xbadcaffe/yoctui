# Current Task

## Task

**ID:** COMPAT-BITBAKE-CANCEL-RUNTIME-001
**Title:** Keep live BitBake cancellation responsive during native events
**Status:** IN_PROGRESS

## Objective

Allow the bridge to receive and execute `cancel_build` while a native BitBake
build is actively producing events, without losing event ordering or blocking
the daemon's IPC loop.

## Dependencies

- `COMPAT-BITBAKE-API-001` — DONE
- `COMPAT-DAEMON-RUNTIME-001` — DONE
- `COMPAT-PROBE-AGGREGATION-001` — DONE

## Relevant files

- `crates/yoctui-bitbake/bridge/yoctui_bridge.py`
- `bridge/tests/test_bridge.py`
- `crates/yoctui-bitbake/src/bridge.rs`
- `crates/yoctui-bitbake/src/backend.rs`
- `crates/yoctui-cli/src/daemon_bitbake.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Native BitBake event polling is incremental and yields to command input; no
  synchronous iterator monopolizes the bridge command loop.
- An accepted daemon `CancelJob` reaches the active BitBake cancellation API
  promptly and produces exactly one typed terminal outcome.
- Event/cancel correlation and ordering remain bounded; late events cannot
  resurrect cancelled work.
- Status, Doctor, and a second local client remain responsive during event
  bursts and cancellation.
- Fake bridge/backend/daemon tests cover active cancellation, event flood,
  rejection/failure, terminal deduplication, and shutdown.
- A live Wrynose build is cancelled through daemon IPC and reaches a terminal
  cancelled/failed state without force-killing the daemon.

## Verification

```bash
python3 -m pytest bridge/tests -k compatibility_cancellation
cargo test -p yoctui --bin yoctui daemon_compatibility_cancellation
./scripts/verify-roadmap.sh
```
