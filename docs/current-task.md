# Current Task

## Task

**ID:** RELVAL-POKY-001
**Title:** Validate Yoctui against a freshly cloned Poky release
**Status:** BLOCKED

## Objective

Run the fresh-Poky release workflow in a network-enabled environment. This is
blocked locally because DNS cannot resolve github.com.

## Verification

```bash
./scripts/test-fresh-poky.sh
```

## Definition of done

- Exact Poky/BitBake/host/artifact evidence is required before unblocking.

## Next task

After completion, select `RELVAL-README-001`.
