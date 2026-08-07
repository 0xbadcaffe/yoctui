# Current Task

## Task

**ID:** UTIL-LAYERS-001
**Title:** Cover common bitbake-layers operations
**Status:** NOT_STARTED

## Objective

Add typed show, overlay, append, dependency, flatten, create, add, remove,
save-build-conf, and layerindex workflows with mutation previews and refresh.

## Verification

```bash
cargo test -p yoctui -- utility_bitbake_layers
./scripts/test-utility-fixtures.sh bitbake-layers
```

## Definition of done

- bitbake-layers operations use typed forms, bounded destinations, and
  confirmation for mutations.

## Next task

After completion, select `UTIL-PKGDATA-001`.
