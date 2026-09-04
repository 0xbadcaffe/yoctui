# Current Task

## Task

**ID:** UX-LIVE-NAV-LOG-CANCEL-001
**Title:** Fix live Navigator, log, and cancellation regressions
**Status:** DONE

## Objective

Keep collapsed Navigator headings reachable after moving away, preserve typed
BitBake log context across daemon IPC, make live logs scroll immediately, and
bound cancellation when BitBake does not publish a terminal event.

## Dependencies

- UX-DASHBOARD-FOCUS-DENSITY-001 — DONE

## Definition of done

- Collapsed Navigator roots remain part of keyboard traversal, can be selected
  again after moving away, and reopen with Right, `l`, or Enter.
- Daemon log records retain recipe, task, source path, and build context so the
  Dashboard task log and Logs workspace render authoritative live output.
- Opening Logs places focus on its scrollable workspace; Up/Down, PageUp/
  PageDown, Home/End, and mouse wheel pause follow and move the visible row.
- Accepted cancellation reaches a terminal state within a bounded deadline,
  even when BitBake never emits its terminal event.
- Focused, protocol, workspace, Clippy, docs, and roadmap checks pass in
  version 0.1.21.

## Verification

```bash
cargo test -p yoctui-model collapsed_navigator_root_can_be_reselected_and_reopened
cargo test -p yoctui-app daemon_log_context_survives_snapshot_and_live_event_mapping
cargo test -p yoctui-app logs_open_with_scrollable_workspace_focus
cargo test -p yoctui daemon_cancellation_times_out_to_one_terminal_event
cargo test -p yoctui-ui
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```

M44 is complete in v0.1.21. Collapsed Navigator group surrogates remain in
keyboard traversal and reopen normally. Daemon log snapshots and live events
retain recipe, task, path, and build identity; snapshot resynchronization keeps
a paused log viewport stable instead of forcing it back to follow mode. Logs
opens with workspace focus and the full bounded vertical key vocabulary.
Cancellation now has a three-second terminal-event deadline, bounded server
termination, and one synthetic exit-130 cancellation result when BitBake does
not close the lifecycle itself. The focused Python and Rust regressions, full
workspace suite, Clippy, and documentation/completion gates pass.
