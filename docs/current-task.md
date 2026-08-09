# Current Task

## Task

**ID:** ENV-CLI-001
**Title:** Start unconfigured sessions without an implicit build directory
**Status:** IN_PROGRESS

## Objective

Make build-dir optional for interactive startup and create an unconfigured app
instead of treating the current directory as a build.

## Verification

```bash
cargo test -p yoctui build_environment
```

## Definition of done

- No-argument startup creates an unconfigured app instead of using cwd.
- Explicit `--build-dir` and persisted recent build directories retain legacy
  behavior.

## Next task

Finish `ENV-CLI-001`, then implement `ENV-CONNECT-001`.

## Terminal handoff

`ENV-UI-001` completed; `ENV-CLI-001` is in progress.
