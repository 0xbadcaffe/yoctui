# Current task

## Active task

**ID:** TEST-RENDER-001
**Title:** Render responsive Testing workspace

## Objective

Render the complete typed Testing launch, result, comparison, and export
workflow at every supported terminal width without parsing backend text.

## Required work

1. Inspect the existing Testing renderer and reuse every already implemented
   launch, result, dialog, focus, theme, and responsive primitive.
2. Render Launches, Results, and Comparison views from typed model state,
   including exact active configuration identity and capability availability.
3. Render lifecycle/output, empty, partial, failure, cancellation, timeout,
   loss, stale-safe selection, suite/case drill-down, metadata, limitations,
   related logs, and regression categories without inferring authority.
4. Render all Testing launch, cancellation, import, comparison, and JUnit
   dialogs with trapped focus, exact previews, disabled explanations, and
   specified footer shortcuts.
5. Preserve semantic selection, status, severity, progress, and comparison
   styling in every theme and in no-color mode.
6. Add Ratatui TestBackend coverage for every state family, dialog, theme, and
   responsive boundary, including 80x24 and too-small terminals.

## Definition of done

- Every Testing view and dialog in `docs/ui-spec.md` renders typed state.
- Wide, medium, narrow, 80x24, and too-small layouts are deterministic and
  panic-free.
- Missing tools, unavailable prerequisites, empty data, partial limitations,
  and all terminal outcomes remain visibly distinct.
- Widgets do not parse raw BitBake, test-runner, resulttool, or filesystem
  text as authority.
- Focus and footer behavior match the authoritative UI specification.

## Verification

```bash
cargo test -p yoctui-ui test_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
