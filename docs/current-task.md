# Current Task

## Task

**ID:** UI-LIVE-RECOVERY-001
**Title:** Complete live workspace usability recovery
**Status:** IN_PROGRESS

## Objective

Consolidate final acceptance for safe startup, live metadata discovery, visual
theme behavior, explicit focus routing, and the real Poky colored workbench.

## Dependencies

- `UI-LIVE-STARTUP-001` — DONE
- `UI-LIVE-DISCOVERY-001` — DONE
- `UI-LIVE-POKY-001` — DONE

## Relevant files

- `crates/yoctui-cli/src/main.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `scripts/test-tui-snapshots.sh`
- `scripts/test-live-workbench.sh`
- `docs/task-registry.toml`
- `docs/implementation-status.md`

## Definition of done

- Every M14 child task is DONE.
- Formatting, workspace tests, Clippy, bridge tests, and roadmap checks pass.
- The terminal handoff returns to the independent host perf-policy blocker.

## Verification

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```
