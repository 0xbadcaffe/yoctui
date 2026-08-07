# Current Task

## Task

**ID:** UTIL-RUNNER-001
**Title:** Add a shared safe external-utility runner
**Status:** NOT_STARTED

## Objective

Implement typed executable identity, exact argv previews, environment/cwd
policy, bounded output, timeout, cancellation, and persistent job history.

## Verification

```bash
cargo test -p yoctui-model utility_runner
cargo test -p yoctui-bitbake utility_runner
cargo test -p yoctui-app utility_runner
cargo test -p yoctui -- utility_runner
```

## Definition of done

- Common and expert utility execution is shell-free, bounded, cancellable, and
  represented by typed jobs with exact previews.

## Next task

After completion, select `UTIL-MENU-001`.
