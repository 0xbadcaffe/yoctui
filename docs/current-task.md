# Current Task

## Task

**ID:** COMPAT-WORKSPACE-001
**Title:** Apply capabilities to all workspaces
**Status:** IN_PROGRESS

## Objective

Close the aggregate workspace acceptance gate by verifying that every
Navigator destination and action consumes the centralized capability snapshot.

## Dependencies

- `COMPAT-WORKSPACE-CATALOG-001` — DONE
- `COMPAT-WORKSPACE-MODEL-001` — DONE
- `COMPAT-WORKSPACE-APP-001` — DONE

## Relevant files

- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-model/src/workspace_compatibility.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Every Navigator destination and every typed effect is present in the closed
  workspace requirement inventory.
- Model projection, snapshot replacement/invalidation, dialog revalidation,
  and unavailable-effect rollback pass their focused tests.
- App/CLI snapshot lifecycle, shared daemon/local enforcement, daemon-owned
  probe suppression, and no-spawn routing pass their focused tests.
- The parent gate is marked DONE only after its three children and aggregate
  verification pass.

## Verification

```bash
cargo test -p yoctui-app compatibility_workspace
cargo test -p yoctui-model compatibility_workspace
./scripts/verify-roadmap.sh
```
