# Current task

## Active task

**ID:** CONFIG-COMPARE-001
**Title:** Compare typed configuration values

## Objective

Compare the selected variable's loaded global and active recipe-scoped values
without parsing display text or fabricating missing data.

## Required work

1. Inspect current scoped identity/detail state, dialog/focus patterns,
   responsive rendering, action availability, and tests.
2. Add a typed comparison record containing variable, global identity, recipe
   identity, effective values, unexpanded values, and explicit equal,
   different, or unavailable outcomes for each field.
3. Make comparison available only when a recipe scope is active and both exact
   global/scoped details are loaded. Loading, failed, not-loaded, missing
   values, and disappeared scope receive precise disabled explanations.
4. Open a focus-trapping read-only comparison dialog from a documented
   shortcut; `Esc` or `Enter` closes and restores the exact prior pane.
5. Render both scopes and field outcomes safely across responsive modes,
   preserving long values through wrapping rather than truncating identity.
6. Add reducer, app/input, and TestBackend tests named `config_compare`,
   including equal, different, unavailable, stale, and narrow states.
7. Update `docs/ui-spec.md` for the comparison interaction.

## Definition of done

- Comparison is typed and tied to exact global/recipe identities.
- Equal/different/unavailable outcomes are honest and deterministic.
- Partial/stale states remain inert with visible reasons.
- Dialog focus and responsive rendering are covered.
- Task-specific and baseline verification pass.
- Parent action task/status and the next eligible task are updated.

## Verification

```bash
cargo test -p yoctui-model config_compare
cargo test -p yoctui-app config_compare
cargo test -p yoctui-ui config_compare
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`CONFIG-EDIT-001 — Add previewed configuration editing and refresh`
