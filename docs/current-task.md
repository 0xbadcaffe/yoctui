# Current Task

## Task

**ID:** UTIL-CORE-001
**Title:** Integrate common BitBake and environment utilities
**Status:** NOT_STARTED

## Objective

Provide typed or contextual workflows for core BitBake/environment utilities,
with setup capabilities distinct from arbitrary child jobs.

## Verification

```bash
cargo test -p yoctui -- utility_core
./scripts/test-utility-fixtures.sh core
```

## Definition of done

- Core utility workflows remain typed or capability-aware and setup commands
  are never exposed as arbitrary jobs.

## Next task

After completion, select `UTIL-ADVANCED-001`.
