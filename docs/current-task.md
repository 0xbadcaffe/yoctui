# Current task

## Active task

**ID:** QA-RENDER-001
**Title:** Render responsive QA workspace

## Objective

Render the complete typed Recipe & Kernel and Layer QA workspace, findings,
sessions, limitations, and modal lifecycle at every supported responsive
breakpoint and theme without parsing backend text.

## Required work

1. Inspect the typed QA state/selectors, shared responsive shell, neighboring
   Testing/Security renderers, dialog helpers, semantic theme roles, and the
   authoritative QA UI specification before writing code.
2. Add QA as a first-class Navigator workspace after Security and render the
   `Recipe & Kernel` and `Layer QA` views selected by typed state.
3. Render capability loading/available/partial/failed states, exact scope,
   catalog or configured-layer rows, typed status/counts, report availability,
   search/filter state, selection, and explicit empty states.
4. Render the Inspector with exact capability, provider/layer, session/output,
   report/fingerprint, finding/source/rule/suggestion/metadata, and limitation
   details; missing fields must say `unavailable`.
5. Render operation, layer-operation, import, and cancellation dialogs as
   focus-trapping responsive overlays with exact indexed previews and stable
   disabled reasons.
6. Render every success, failure, nonzero, cancelled, timed-out, lost, partial,
   malformed, missing, permission, and stale state distinctly using semantic
   text plus color/attributes.
7. Render the specified full and compact QA footer shortcuts and ensure long
   paths, vectors, findings, metadata, output, and limitations wrap/bound
   safely at every responsive breakpoint, including 80×24, all themes, and
   no-color mode.
8. Add Ratatui `TestBackend` coverage for both views, every capability/report/
   session/dialog family, responsive boundaries, selection/drill, themes, and
   no-color behavior. Do not implement adapter or CLI polling logic here.

## Definition of done

- Widgets consume only typed QA model state and emit no parsed authority.
- Both QA views, Inspector, dialogs, explicit states, and footer match
  `docs/ui-spec.md`.
- All supported terminal sizes and themes render without panic or semantic
  loss.
- Focused UI and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-ui qa_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
