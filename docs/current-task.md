# Current task

## Active task

**ID:** CONFIG-SCOPE-001
**Title:** Add recipe-scoped configuration inspection

## Objective

Let users switch the selected variable between global and supported recipe
scope while preserving each typed detail identity independently.

## Required work

1. Inspect current global selection identity, recipe inventory, lazy detail
   execution, dialogs/focus, responsive Inspector, copy/source availability,
   and tests without duplicating behavior.
2. Add a typed scope picker containing global scope and authoritative recipe
   names from the current workspace. Empty recipe inventories must keep global
   inspection available and explain the limitation.
3. Store the selected optional recipe scope separately from the global summary
   table and derive the exact `VariableIdentity` from variable plus scope.
4. Confirming scope requests authoritative detail for that identity through
   the existing backend effect; global and recipe-scoped loading, error, and
   loaded records remain independent.
5. Ignore stale responses for rendering/action availability, retain selected
   scope only while its recipe remains in refreshed inventory, and return to
   global scope if it disappears.
6. Render scope and shortcut availability across responsive modes; copy and
   source routes must automatically use the active scoped identity.
7. Add reducer, app/input, CLI integration, and TestBackend tests named
   `config_scope`, including global fallback and partial/error states.
8. Update `docs/ui-spec.md` for scope selection and refresh behavior.

## Definition of done

- Global and recipe-scoped detail are independently addressable and retained.
- Scope selection uses authoritative recipe inventory and typed effects.
- Refresh/stale/empty/failure behavior is deterministic and honest.
- Existing copy/source actions follow the selected scope.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model config_scope
cargo test -p yoctui-app config_scope
cargo test -p yoctui-ui config_scope
cargo test -p yoctui -- config_scope
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`CONFIG-COMPARE-001 — Compare typed configuration values`
