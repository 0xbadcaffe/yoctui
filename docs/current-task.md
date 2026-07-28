# Current task

## Active task

**ID:** WIC-UI-RENDER-001
**Title:** Render responsive Wic creation and output UI

## Objective

Render typed Wic capability, kickstart partitions, latest creation job,
generated outputs, exact creation/confirmation/cancellation dialogs, and
footer hints responsively in the Images workspace.

## Required work

1. Inspect the Images Inspector, QEMU rendering, semantic theme roles, dialog
   overlay helpers, and responsive TestBackend fixtures before editing.
2. Render Wic capability/executable/readiness or exact disabled reason before
   selected-artifact metadata without hiding QEMU state.
3. Render the selected kickstart source/typed partition summary and explicit
   dynamic/unsupported limitations without deriving new data in widgets.
4. Render the latest Wic session request, lifecycle/timestamps, stream-tagged
   bounded output/drop/truncation/result/error, then generated-output rows with
   exact kind/path/size/time and visible selection.
5. Render focus-trapped creation, exact command preview, and cancellation
   overlays at every supported breakpoint; show read-only/editing/choice state,
   validation, partition preview, and complete exact argv.
6. Update the Images footer with `W create Wic`, shared `x cancel`, `[/] select
   output`, and `O open output` while retaining QEMU and artifact hints.
7. Add `wic_workspace` TestBackend coverage for every capability/inventory/job
   state, all terminal outcomes, modal focus, long content, themes, 80x24,
   medium/wide, and below-minimum terminals.
8. Run focused and baseline checks, then mark the child done and hand off to
   `WIC-UI-CLI-001`.

## Definition of done

- Every typed Wic state has explicit responsive rendering.
- Dialogs trap focus and remain usable at 80x24.
- Widgets consume typed state without parsing Wic or kickstart text.
- Footer hints exactly match implemented shortcuts.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-ui wic_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
