# Current Task

## Task

**ID:** COMPAT-UI-001
**Title:** Expose capability state clearly in the UI
**Status:** IN_PROGRESS

## Objective

Expose centralized capability state throughout the interface without hiding
useful unavailable actions or cluttering normal workflows with version detail.

## Dependencies

- `COMPAT-WORKSPACE-001` — DONE
- `COMPAT-PROTOCOL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/`
- `crates/yoctui-app/src/`
- `crates/yoctui-ui/src/`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Useful unavailable actions remain visible but disabled.
- Every disabled or limited action exposes the exact centralized reason and,
  where useful, its required capability/tool or maintained alternative.
- Normal workspace flow is not cluttered with release/version detail.
- A bounded Environment/Compatibility inspector exposes detected identity,
  capability summary, exact evidence/reasons, and selected implementations.
- Dynamic snapshot changes update action state and reasons without panic or
  stale launches; TestBackend and app tests cover responsive behavior.

## Verification

```bash
cargo test -p yoctui-ui compatibility
cargo test -p yoctui-app compatibility_ui
./scripts/test-tui-snapshots.sh
./scripts/verify-roadmap.sh
```
