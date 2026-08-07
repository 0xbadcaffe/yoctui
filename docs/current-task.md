# Current Task

## Task

**ID:** UTIL-ADVANCED-001
**Title:** Integrate advanced QA, maintenance, and release utilities
**Status:** NOT_STARTED

## Objective

Expose capability-aware menus for advanced QA, maintenance, release, CVE/SPDX,
cache, archive, pull-request, and service diagnostics.

## Verification

```bash
cargo test -p yoctui -- utility_advanced
./scripts/test-utility-fixtures.sh advanced
```

## Definition of done

- Advanced utility workflows preserve typed safety, capability detection,
  bounded evidence, and managed-service restrictions.

## Next task

After completion, select `UTIL-DOC-001`.
