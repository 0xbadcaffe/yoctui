# Current Task

## Task

**ID:** ENV-UI-001
**Title:** Render build environment settings and setup shell handoff
**Status:** NOT_STARTED

## Objective

Render the focus-trapped setup form, exact previews, disabled reasons,
responsive states, and embedded-shell handoff.

## Verification

```bash
cargo test -p yoctui-ui build_environment
cargo test -p yoctui-app build_environment
```

## Definition of done

- Setup fields, connection states, and exact previews render responsively.
- Build actions show the shared disabled reason until connected.
- Interactive setup-shell handoff traps focus and restores it safely.

## Next task

Implement `ENV-UI-001`.

## Terminal handoff

`ENV-CLONE-001` completed; `ENV-UI-001` is eligible to start.
