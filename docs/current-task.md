# Current task

## Active task

**ID:** SIG-UI-001
**Title:** Integrate signature dump and comparison workflows

## Objective

Add an app-owned, responsive signature workspace launched from the selected
recipe, with authoritative task selection, background dump/comparison
execution, explicit states, deterministic navigation, and safe return to the
Recipes workspace.

## Required work

1. Inventory existing recipe shortcuts/dialogs, recipe task metadata, workspace
   navigation/focus, responsive render helpers, footer hints, editor effects,
   background polling, and current signature model/adapter behavior.
2. Update `docs/ui-spec.md` before implementing the intentional behavior:
   - `Z` from Recipes opens a focus-trapping signature task picker populated
     only by the selected recipe's authoritative task metadata;
   - `Enter` loads the chosen task and opens a dedicated signature workspace;
   - `Esc` returns to Recipes when idle and requests cancellation while an
     operation is running;
   - `Up`/`Down` selects exact historical signature identities, `1`/`2` assigns
     comparison sides, `c` compares, `r` refreshes, and `e` opens the selected
     recipe provider;
   - footer hints and explicit loading/empty/partial/failure content are always
     visible when space permits.
3. Add a typed Signature screen and task-picker state without exposing it as a
   duplicate navigator entry. Preserve exact recipe/task identity and prior
   workspace return context.
4. Gate task selection on current authoritative recipe metadata. Empty, stale,
   invalid, or unavailable selections remain inert with actionable notices.
5. Keep signature input mapping in `yoctui-app`. Dialogs trap focus, selection
   is identity-stable, incomplete/identical comparison sides cannot launch,
   and stale results cannot replace a newer request.
6. Render bounded signature records, selected variable/task-dependency detail,
   comparison-side markers, and categorized differences from typed state only.
   Wide layouts may use multiple panes; narrow layouts stack or compact safely
   and never panic.
7. Run dump and comparison adapters as cancellable Tokio background work so
   terminal drawing and navigation remain responsive. Convert results and
   failures through typed backend events/actions; never parse tool output in
   CLI, app, or UI code.
8. Route provider editing through the existing validated editor lifecycle.
   Signature artifacts remain read-only data and are not opened as source text.
9. Add tests named `signature_workspace` for reducer lifecycle, picker focus,
   input mapping, disabled reasons, background result/failure/cancellation,
   exact identity correlation, wide/narrow/very-small TestBackend rendering,
   footer hints, explicit states, differences, and provider navigation.
10. Update `docs/architecture.md` for background signature coordination if its
    ownership boundary changes.

## Definition of done

- A selected recipe can choose an authoritative task and enter the signature
  workspace without launching a BitBake build task.
- Typed dumps and comparisons run in the background, remain cancellable, and
  preserve exact correlated state.
- All required states, actions, hints, and responsive layouts are tested.
- Provider editing uses the existing safe editor boundary.
- Focused, parent-gate, and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model signature_workspace
cargo test -p yoctui-app signature_workspace
cargo test -p yoctui-ui signature_workspace
cargo test -p yoctui -- signature_workspace
cargo test -p yoctui-app signature
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`PKG-001 — Package data browser`
