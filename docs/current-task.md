# Current Task

## Task

**ID:** DOC-README-002
**Title:** Simplify the Poky README quickstart
**Status:** DONE

## Objective

Final completed task: provide one Poky build-environment path and explicitly
set and pass `BUILDDIR` to `oe-init-build-env`.

## Verification

```bash
./scripts/test-readme-quickstart.sh
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```

## Definition of done

- README presents one guarded Poky setup path.
- `BUILDDIR` is set before and passed to `oe-init-build-env`.
- Documentation checks pass.

## Next task

## Terminal handoff

All registry tasks are complete.
