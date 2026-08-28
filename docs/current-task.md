# Current Task

## Task

**ID:** UX-CONCEPT-RASTER-001
**Title:** Render deterministic PNGs from production cells
**Status:** IN_PROGRESS

## Objective

Convert the six exact production cell/style scenes into deterministic PNG
review artifacts using a pinned font and renderer, with provenance and
checksums that can be reproduced in CI.

## Dependencies

- UX-CONCEPT-ERRORS-001 — DONE
- UX-CONCEPT-ROOTFS-001 — DONE
- UX-CONCEPT-EDITOR-MENU-001 — DONE

## Definition of done

- The renderer consumes exact production cell/style goldens.
- Font identity and rendering parameters are pinned and recorded.
- All six app-derived PNGs are deterministic and checksummed.
- Check mode rejects stale, missing, or non-reproducible artifacts.

## Verification

```bash
./scripts/render-m22-concept-screenshots.sh --check
python3 scripts/test-m22-concept-raster.py
```
