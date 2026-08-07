# Current Task

## Task

**ID:** RELVAL-POKY-001
**Title:** Validate Yoctui against a freshly cloned Poky release
**Status:** NOT_STARTED

## Objective

Validate a pinned fresh Poky checkout with an isolated qemux86-64 build
directory, doctor/bridge inspection, and bounded core-image-minimal workflow.

## Verification

```bash
./scripts/test-fresh-poky.sh
```

## Definition of done

- Fresh-Poky evidence records exact revisions, host, commands, artifacts, and
  live workflow outcomes without generalizing fixture results.

## Next task

After completion, select `RELVAL-README-001`.
