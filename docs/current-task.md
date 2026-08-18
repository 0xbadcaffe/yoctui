# Current Task

## Task

**ID:** COMPAT-WORKSPACE-MODEL-001
**Title:** Project workspace availability and revalidate state
**Status:** IN_PROGRESS

## Objective

Derive every workspace/action availability result and exact reason from one
current capability snapshot, and safely revalidate model state when that
snapshot changes.

## Dependencies

- `COMPAT-WORKSPACE-CATALOG-001` — DONE

## Relevant files

- `crates/yoctui-model/src/workspace_compatibility.rs`
- `crates/yoctui-model/src/daemon_state.rs`
- `crates/yoctui-model/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Pure projection returns available, limited, unavailable, unsupported, or
  unknown plus exact capability reasons for destination and effect policies.
- All-of requirements name every missing capability; any-of requirements
  explain why no alternative is usable.
- Absent authority fails closed for environment-backed actions while local
  presentation and cancellation remain usable.
- Newer snapshots replace current authority; stale/equal conflicting updates
  are ignored or rejected.
- Snapshot changes preserve valid selections and close or revalidate dialogs
  whose launch capability is no longer enabled.
- The model rejects an unavailable effect before emission and reports its
  exact reason without applying release/version policy.
- Tests cover full, partial, absent, unsupported, stale, changing, and dialog
  revalidation states without panics.

## Verification

```bash
cargo test -p yoctui-model compatibility_workspace_model
./scripts/verify-roadmap.sh
```
