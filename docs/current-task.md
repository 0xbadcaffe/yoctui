# Current Task

## Task

**ID:** UTIL-CATALOG-001
**Title:** Define a versioned Yocto utility capability catalog
**Status:** NOT_STARTED

## Objective

Inventory supported Yocto utilities and classify typed workflows, expert
launchers, informational/internal entries, and intentional exclusions.

## Verification

```bash
test -s docs/utility-catalog.md
cargo test -p yoctui-model utility_catalog
./scripts/verify-utility-coverage.sh --catalog-only
```

## Definition of done

- The catalog is versioned, non-empty, and coverage verification passes.

## Next task

After completion, select `UTIL-RUNNER-001`.
