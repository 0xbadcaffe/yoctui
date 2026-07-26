# Current task

## Active task

**ID:** CONFIG-COPY-001
**Title:** Add typed configuration value copy actions

## Objective

Allow users to copy the selected variable's authoritative effective or
unexpanded value through typed actions and the existing clipboard boundary.

## Required work

1. Inspect existing selected-variable identity, detail lifecycle, clipboard
   effect execution, notifications, input mapping, footer/Inspector text, and
   tests without duplicating behavior.
2. Add separate typed actions for copying the selected effective and
   unexpanded values.
3. Resolve values only from loaded detail for the exact selected
   `VariableIdentity`; never copy summary or stale scoped data as authoritative
   detail.
4. Emit the existing typed clipboard effect on success. Missing selection,
   loading, failed/not-loaded detail, and absent values remain inert with exact
   disabled explanations.
5. Map documented shortcuts and show their availability in responsive
   Configuration rendering.
6. Add reducer, app/input, CLI, and TestBackend tests named `config_copy`.
7. Update `docs/ui-spec.md` for the new shortcuts and behavior.

## Definition of done

- Effective and unexpanded copy routes are typed and identity-safe.
- Partial states never copy fallback or fabricated values.
- Shortcuts and disabled reasons render across responsive modes.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model config_copy
cargo test -p yoctui-app config_copy
cargo test -p yoctui-ui config_copy
cargo test -p yoctui -- config_copy
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`CONFIG-SOURCE-001 — Add authoritative configuration source selection`
