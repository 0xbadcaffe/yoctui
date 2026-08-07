# Current Task

## Task

**ID:** UTIL-DEVTOOL-001
**Title:** Cover common Devtool recipe workflows
**Status:** NOT_STARTED

## Objective

Ensure typed UI coverage for common Devtool recipe/workspace workflows with
exact previews, eligibility reasons, and version-aware expert argv fallback.

## Verification

```bash
cargo test -p yoctui -- utility_devtool
./scripts/test-utility-fixtures.sh devtool
```

## Definition of done

- Devtool common operations use typed forms with capability-aware disabled
  reasons and safe expert argv fallback.

## Next task

After completion, select `UTIL-RECIPETOOL-001`.
