# Current Task

**ID:** ERRORS-HISTORY-VIEWER-001
**Title:** Unify current and historical build errors with an in-workspace log viewer
**Status:** IN_PROGRESS

Implement the authoritative Errors workspace changes in `docs/ui-spec.md`:

- separate Current build and Past builds views with independent selection
- project saved warning/error records and derive resolved state only from a
  newer successful build of the same target and machine
- retain saved log recipe/task/build/path context
- open a bounded, read-only source log inside the Errors workspace
- keep exact current-log navigation and external editor opening as secondary
  actions

Verify with:

```bash
cargo test -p yoctui-model errors_history
cargo test -p yoctui-app errors_history
cargo test -p yoctui-ui errors_history
cargo test -p yoctui --all-features error_log
cargo fmt --all --check
```
