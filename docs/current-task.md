# Current task

## Active task

**ID:** LOG-001
**Title:** Complete bounded searchable logs

## Objective

Complete the Logs workspace specified by `docs/ui-spec.md` while preserving
important diagnostics under bounded retention and backend pressure.

## Required work

1. Inventory existing `LogState`, reducer actions, input routes, retention, and
   rendering before adding behavior.
2. Preserve warnings, errors, cancellation, disconnect, and final-result
   diagnostics when ordinary informational logs are evicted.
3. Keep bounded entry and byte retention with observable dropped/coalesced
   counters.
4. Complete live follow, pause/resume, vertical scrolling, wrap, and
   wrap-disabled horizontal scrolling.
5. Complete incremental search with visible match counts and bounded
   next/previous navigation.
6. Complete severity, recipe, task, and selected-build filters.
7. Add a bounded selected log row and populate the Inspector with timestamp,
   severity, recipe, task, source path, and full multiline content.
8. Add typed effects and input for opening the selected source log in the
   configured editor and copying the selected line/details where supported.
9. Render empty, evicted, paused, searching, narrow, and multiline states
   safely in every theme.
10. Add reducer, input-mapping, integration, and Ratatui `TestBackend` coverage
    named `log`.

## Definition of done

- Important diagnostics survive ordinary-log pressure.
- Retention, drop, and coalescing behavior is bounded and observable.
- Follow, pause, scrolling, wrap, search, and all filters are complete.
- Selection drives full structured Inspector content.
- Source opening and copy actions use typed effects and fail visibly.
- Responsive and multiline rendering never panics.
- Reducer, integration, input-mapping, and TestBackend checks pass.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model log
cargo test -p yoctui-ui log
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`ERR-001 — Complete structured error investigation`
