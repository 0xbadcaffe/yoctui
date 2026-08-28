# Current Task

## Task

**ID:** UX-CONCEPT-LIVE-001
**Title:** Capture every concept workflow on a supported live Yocto host
**Status:** IN_PROGRESS

## Objective

Drive all six concept workflows against one exact Yoctui binary on a supported
Yocto host and retain scenario-attributed interaction, assertion, terminal,
metadata, and raster evidence.

## Dependencies

- UX-CONCEPT-ERRORS-001 — DONE
- UX-CONCEPT-ROOTFS-001 — DONE
- UX-CONCEPT-EDITOR-MENU-001 — DONE
- UX-CONCEPT-TERMINAL-LIVE-001 — DONE

## Definition of done

- One checksummed supported-host binary drives all six named scenarios.
- Every scenario records explicit interactions and observed assertions.
- Terminal, semantic/metadata, and raster artifacts remain attributable.
- Unsupported-host diagnostics cannot satisfy live evidence.

## Verification

```bash
./scripts/test-live-m22-concept-parity.sh
./scripts/verify-live-m22-concept-evidence.sh
```
