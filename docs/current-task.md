# Current Task

## Task

**ID:** UTIL-PKGDATA-001
**Title:** Cover common oe-pkgdata-util package workflows
**Status:** NOT_STARTED

## Objective

Add typed package lookup/list/detail/dependency workflows linked to Packages,
Recipes, Images, files, and dependency navigation.

## Verification

```bash
cargo test -p yoctui -- utility_pkgdata
./scripts/test-utility-fixtures.sh oe-pkgdata-util
```

## Definition of done

- oe-pkgdata-util operations preserve unavailable state when pkgdata is absent
  and expose bounded typed package results.

## Next task

After completion, select `UTIL-CORE-001`.
