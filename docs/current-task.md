# Current task

## Active task

**ID:** ERR-001
**Title:** Complete structured error investigation

## Objective

Turn warnings and errors into the complete structured investigation workspace
specified by `docs/ui-spec.md`, with direct navigation to related logs and
source paths.

## Required work

1. Inventory existing warning/error retention, selection, rendering, input,
   completion notification, and log-jump behavior.
2. Represent normalized category, summary, full message, build session,
   recipe, task, source path/log, event metadata, suggestions, and related
   diagnostic identity as typed model state.
3. Render the specified time, severity, recipe, task, summary, and build
   session columns with bounded selection.
4. Populate the Inspector with complete multiline details, category, source,
   event context, suggested actions, and related entries.
5. Make `Enter` jump to the exact matching log context without replacing the
   user's query with the whole diagnostic message.
6. Open the selected source file/log through a typed editor effect and report
   missing paths visibly.
7. Implement completion notifications for success, warnings-only, errors, and
   cancellation; `Enter` on failure opens the selected Errors context.
8. Preserve structured warnings/errors when ordinary logs are dropped and
   expose any diagnostic loss honestly.
9. Render empty, multiline, narrow, and backend-loss cases safely in every
   theme.
10. Add reducer, input-mapping, integration, and Ratatui `TestBackend` coverage
    named `error`.

## Definition of done

- Warnings and errors are structured, bounded, selectable investigation rows.
- All specified list and Inspector metadata is typed and visible.
- Exact log-context and source navigation work without text parsing.
- Completion outcomes produce distinct actionable notifications.
- Diagnostic retention/loss remains observable under pressure.
- Responsive and multiline rendering never panics.
- Reducer, integration, input-mapping, and TestBackend checks pass.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model error
cargo test -p yoctui-ui error
cargo test -p yoctui-app error
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`LAYERS-001 — Complete lazy Layers tree and contextual Inspector`
