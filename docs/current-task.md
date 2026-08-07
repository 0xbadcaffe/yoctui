# Current Task

## Task

**ID:** SHELL-UI-001
**Title:** Render and operate the embedded shell workspace
**Status:** NOT_STARTED

## Objective

Add a Shell workspace with full-screen focus, visible session/cwd/status,
scrollback, copy/search, multiline-paste confirmation, resize, bounded
sessions, and clear escape/close/restart shortcuts.

## Verification

```bash
cargo test -p yoctui-ui embedded_shell
cargo test -p yoctui-app embedded_shell_input
```

## Definition of done

- Shell rendering and input ownership are visible and responsive across the
  supported terminal breakpoints.

## Next task

After completion, select `SHELL-INTEGRATION-001`.
