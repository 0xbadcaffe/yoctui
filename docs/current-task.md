# Current Task

**ID:** FOCUS-NAVIGATOR-001
**Title:** Keep default focus on Navigator and skip passive panes
**Status:** DONE

Make Navigator the focus owner at interactive startup and after destination
navigation. Tab and Shift+Tab are the only keyboard pane-focus routes, and
focus traversal includes Workspace or Inspector only when the pane exposes a
selectable, scrollable, editable, or terminal-input control. Add model, app,
CLI, responsive UI, and deterministic snapshot coverage; bump the release;
then run the workspace baseline and completion gates.

Version 0.1.108 implements the shared actionable-pane policy across reducer,
keyboard, mouse, responsive switcher, command palette, footer, CLI, and concept
fixtures. Navigator owns startup and destination changes. Tab and Shift+Tab
reach Workspace only where it owns controls; Inspector remains a read-only
projection. The focused suites, all-features workspace baseline, strict Clippy,
bridge tests, documentation checks, and deterministic raster checks pass.
