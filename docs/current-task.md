# Current task

## Active task

**ID:** PALETTE-001
**Title:** Implement the searchable contextual command palette

## Objective

Replace the fixed six-entry command overlay with a typed, searchable command
catalog that reflects the active workspace, build state, selection, and
backend capability while explaining unavailable actions.

## Required work

1. Inventory every currently reachable global/workspace action and its
   availability requirements before defining the catalog.
2. Define typed command identifiers, labels, descriptions, shortcuts,
   contextual visibility, availability, and disabled explanations in the
   model/application boundary.
3. Add a query string with append, backspace, selection, activation, and close
   reducer transitions.
4. Filter case-insensitively across labels, descriptions, and shortcuts while
   preserving deterministic ordering and bounded selection.
5. Keep unavailable commands discoverable and visibly disabled with an exact
   explanation; activation must be inert.
6. Execute available commands through existing typed actions/effects rather
   than duplicating workflow logic.
7. Route palette input before dialogs and workspace input while preserving the
   exact pane return target.
8. Render query, contextual results, descriptions, shortcuts, selection, and
   disabled state safely at wide and narrow supported sizes.
9. Add model reducer, app mapping, CLI routing, and TestBackend coverage for
   filtering, empty results, disabled commands, activation, and focus restore.
10. Update `docs/ui-spec.md` and `docs/architecture.md` for the command catalog
    ownership and availability contract.

## Definition of done

- One typed catalog determines palette content and ordering.
- Search and selection are reducer-owned and deterministic.
- Contextual/unavailable commands explain their state and cannot activate.
- Available entries dispatch existing typed workflows.
- Focus trapping and exact pane restoration remain correct.
- Wide, narrow, theme, and no-color rendering are covered.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model command_palette
cargo test -p yoctui-app command_palette
cargo test -p yoctui-ui command_palette
cargo test -p yoctui -- command_palette
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`BB-002 — Complete the typed backend-to-model event boundary`
