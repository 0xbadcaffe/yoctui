# Current task

## Active task

**ID:** CONFIG-SOURCE-001
**Title:** Add authoritative configuration source selection

## Objective

Open an authoritative defining source for the selected variable from its typed
operation history, including an explicit picker when multiple sources exist.

## Required work

1. Inspect the current summary-provenance `o` route, loaded detail operations,
   dialog/focus patterns, editor lifecycle, relative-path resolution, and
   tests without duplicating behavior.
2. Replace summary-string parsing as the primary route with typed operation
   file/line choices from the exact selected `VariableIdentity`.
3. Open one authoritative source directly; when multiple distinct sources are
   available, open a focus-trapping picker showing operation, file, and line.
4. Resolve relative paths against the active build directory only at the
   filesystem/effect boundary. Reject empty, escaped, missing, stale, or
   unavailable selections with exact explanations.
5. Preserve loading/error/not-loaded/empty distinctions and keep the legacy
   summary provenance visible but disabled as a source action until typed
   detail is loaded.
6. Add reducer, app/input, CLI editor, and TestBackend tests named
   `config_source`, including picker focus and partial states.
7. Update `docs/ui-spec.md` for the source-selection interaction.

## Definition of done

- Source actions use typed operation identities rather than parsing display
  strings.
- Single and multiple defining-source routes are deterministic and safe.
- Dialog focus, cancellation, stale selection, and editor failures are covered.
- Disabled reasons render across responsive modes.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model config_source
cargo test -p yoctui-app config_source
cargo test -p yoctui-ui config_source
cargo test -p yoctui -- config_source
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`CONFIG-SCOPE-001 — Add recipe-scoped configuration inspection`
