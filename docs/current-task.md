# Current task

## Active task

**ID:** DEP-UI-001
**Title:** Integrate dependency and why-built workspace

## Objective

Replace the legacy flat dependency screen with a responsive typed graph
workspace that exposes recipe/task dependencies, reverse context, and an
honest bounded why-built path without parsing backend text.

## Required work

1. Inventory the existing flat dependency renderer, CLI-only key routing,
   legacy selection/open behavior, footer shortcuts, Inspector integration,
   and responsive breakpoints.
2. Reconcile and expand `docs/ui-spec.md` before implementing details: define
   graph row semantics, selection, why-built/reverse presentation, exact
   shortcuts, open behavior, and all not-loaded/loading/empty/partial/failure
   states.
3. Render a deterministic navigable recipe/task list or tree from
   `DependencyGraphState`; distinguish build, runtime, and task context without
   inventing edges or percentages.
4. Drive the persistent Inspector from the selected typed identity. Show exact
   root/node identity, provider/log availability, incoming/reverse and outgoing
   edge context, limitations, and one bounded shortest why-built path with
   explicit unreachable/limit outcomes.
5. Preserve selection by typed identity across refresh and keep navigation
   bounded at every terminal size. Loading or failed refreshes must not expose
   stale data as current.
6. Route dependency input through `yoctui-app` typed actions rather than
   CLI-only conditionals. Recipe navigation, provider opening, and task-log
   opening must use only authoritative typed identities/paths and explain
   unavailable actions.
7. Preserve the persistent shell and semantic focus rules in wide, medium,
   narrow, and too-small layouts. Long identities/paths and partial graphs must
   wrap or truncate safely and never panic.
8. Add reducer, app input, CLI integration, and Ratatui `TestBackend` tests
   named `dependency_workspace` for all states, selection refresh/disappearance,
   reverse/path/cycle/bound outcomes, action availability, focus, and boundary
   widths.
9. Remove legacy flat rendering/routing only after typed compatibility tests
   prove the migration; keep the protocol legacy fallback in the adapter.

## Definition of done

- The Dependencies workspace consumes only typed model graph state.
- Recipe/task, reverse-edge, limitation, and why-built information is visible
  and honest at every supported breakpoint.
- All actions are typed, identity-correlated, and path-authoritative.
- No widget or CLI path parses raw BitBake/dot/process output.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model dependency_workspace
cargo test -p yoctui-app dependency_workspace
cargo test -p yoctui-ui dependency_workspace
cargo test -p yoctui -- dependency_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`SIG-001 — Signature dump and comparison workflows`
