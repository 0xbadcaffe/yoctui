# Current Task

## Task

**ID:** MAINT-RENDER-001
**Title:** Render responsive Maintenance workspace

## Objective

Render the authoritative typed Maintenance state as a first-class responsive
workspace with four fixed views, exact capability and identity details,
operation/session evidence, safe dialogs, themes, and narrow-terminal behavior.

## Required work

1. Inspect the existing shell, responsive helpers, Maintenance model, dialog
   queue, theme roles, and TestBackend patterns before adding renderer code.
2. Render `Sstate`, `Services`, `Release`, and `Integrations` in fixed order,
   preserving each view's typed selection and clearly distinguishing available,
   partial, disabled, loading, failed, and unavailable state.
3. In wide mode render the capability/operation list plus exact Inspector
   details; use the standard Inspector overlay in medium mode and standard pane
   switcher in narrow mode. Too-small terminals must use the standard safe
   message and never panic.
4. Render canonical metadata, tool/interface identity, service endpoint and
   process observations, exact indexed preview arguments and limitations,
   retained bounded output, lifecycle/terminal outcomes, and evidence identity.
   Widgets must not parse raw process or BitBake text.
5. Render the exact shared and view-specific footer shortcuts from section 20
   of `docs/ui-spec.md`; disabled actions stay visible with typed reasons.
6. Render Maintenance confirmations through the existing modal queue, trapping
   focus at 80x24. Distinguish ordinary, cleanup-phrase, destructive,
   network-push, cancellation, and terminal outcome meaning without inventing
   new keys or bypassing typed dialog state.
7. Add Ratatui TestBackend coverage for every view and major lifecycle state,
   selection/Inspector detail, modal focus, all responsive boundaries, every
   theme, and no-color semantic distinctions.
8. Update `docs/ui-spec.md` in this commit only if implementation reveals an
   intentional UI behavior change; do not silently diverge from it.

## Definition of done

- Maintenance renders only typed model state at all supported breakpoints.
- Every required view, state, footer, dialog, theme, and no-color distinction
  has focused TestBackend coverage.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-ui maintenance_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/ui-spec.md` in this commit for any intentional UI behavior change.
- Update `docs/architecture.md` only if the rendering boundary changes.
- Mark `MAINT-RENDER-001` `DONE` only after verification passes.
- Update `docs/implementation-status.md`.
- Replace this file with the next eligible highest-priority Maintenance task.

## Next task

`MAINT-CLI-001`
