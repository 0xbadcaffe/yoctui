# Current Task

## Task

**ID:** UI-LIVE-STARTUP-001
**Title:** Restore safe metadata-capable interactive startup
**Status:** IN_PROGRESS

## Objective

Prevent launch/test overrides from poisoning later sessions and make a normal
explicit Poky build-directory launch load typed metadata through the bridge
without an obscuring daemon-unavailable notice.

## Dependencies

- `UI-LIVE-RECOVERY-SPEC-001` — DONE

## Relevant files

- `crates/yoctui-cli/src/main.rs`
- `scripts/test-tui-snapshots.sh`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/task-registry.toml`
- `docs/implementation-status.md`

## Definition of done

- Legacy session backend state does not override the default bridge.
- `--no-color` does not rewrite the stored interactive color preference.
- Snapshot subprocesses use private XDG config/state/runtime roots.
- Missing daemon status does not obscure the local interactive workbench.
- Focused tests and the roadmap gate pass.

## Verification

```bash
cargo test -p yoctui -- startup_session
./scripts/test-tui-snapshots.sh
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
