# Current Task

## Task

**ID:** UTIL-DOC-001
**Title:** Document utility coverage, safety, and extension points
**Status:** NOT_STARTED

## Objective

Document every utility's typed coverage, expert-mode availability, supported
versions, environment, risk, output handling, and intentional exclusions.

## Verification

```bash
test -s docs/utility-catalog.md
test -s docs/operator-guide.md
./scripts/check-docs.sh
./scripts/verify-utility-coverage.sh
```

## Definition of done

- Catalog and operator documentation match utility coverage and safety policy.

## Next task

After completion, select `SHELL-MODEL-001`.
