# Current Task

## Task

**ID:** COMPAT-TEST-UI-001
**Title:** Test dynamic feature loading and unloading
**Status:** IN_PROGRESS

## Objective

Use typed compatibility snapshots to prove that live capability replacement
updates model, app, and rendered UI behavior safely without stale actions or
invalid process launches.

## Dependencies

- `COMPAT-TEST-FIXTURES-001` — DONE
- `COMPAT-UI-001` — DONE

## Relevant files

- `crates/yoctui-model/src/compatibility_ui.rs`
- `crates/yoctui-model/src/workspace_compatibility.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-bitbake/src/compatibility_fixtures.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Replacing a capability snapshot while running enables and disables actions
  immediately from the centralized projection.
- Existing selections remain valid or reconcile safely and open dialogs close
  or revalidate when their authority disappears.
- Stale snapshot responses are ignored and disabled reasons update from the
  newest accepted generation.
- Model, app, and TestBackend tests cover replacement, invalidation, no panic,
  and no invalid command launch.

## Verification

```bash
cargo test -p yoctui-model compatibility_dynamic
cargo test -p yoctui-app compatibility_dynamic
cargo test -p yoctui-ui compatibility_dynamic
./scripts/verify-roadmap.sh
```
