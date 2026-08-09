# Current Task

## Task

**ID:** UX-ENV-NAV-001
**Title:** Add dedicated Build environment Navigator workspace
**Status:** NOT_STARTED

## Objective

Add the Screen/Navigator entry, Navigator startup focus, visible disconnected
state, and removal of the environment row from general Settings.

## Verification

```bash
cargo test -p yoctui-model build_environment
cargo test -p yoctui-app build_environment
cargo test -p yoctui-ui build_environment
```

## Definition of done

- Build environment is a dedicated Navigator destination.
- Unconfigured startup focuses Navigator and selects the destination.
- General Settings contains only visual/log preferences.

## Next task

Implement `UX-ENV-NAV-001`.

## Terminal handoff

`UX-GOV-001` completed; `UX-ENV-NAV-001` is eligible to start.
