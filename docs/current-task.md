# Current Task

## Task

**ID:** UTIL-RECIPETOOL-001
**Title:** Add common Recipetool recipe workflows
**Status:** NOT_STARTED

## Objective

Add typed create, appendfile, newappend, setvar, and source-analysis forms with
layer-aware paths, previews, protection, refresh, and expert argv fallback.

## Verification

```bash
cargo test -p yoctui -- utility_recipetool
./scripts/test-utility-fixtures.sh recipetool
```

## Definition of done

- Recipetool common operations use typed forms with protected destinations and
  safe expert argv fallback.

## Next task

After completion, select `UTIL-LAYERS-001`.
