# Current Task

## Task

**ID:** UX-VISUAL-LIVE-001
**Title:** Recapture all six styled scenarios from the current release binary
**Status:** DONE

## Objective

The historical monochrome/stale capture set has been replaced with six
current-release, style-faithful screens from a single supported-host run.

## Dependencies

- UX-VISUAL-SHELL-001 — DONE
- UX-LIVE-STYLE-001 — DONE

## Definition of done

- Six current-release supported-host Yoctui PNGs are directly visible under
  `docs/design` and materially resemble their M21 counterparts.
- Exact live cell data preserves foreground, background, and bold styles.
- One machine-readable manifest retains the exact live capture identity.
- Documentation, visual parity, and completion gates pass.

## Verification

One optimized 0.1.3 binary with SHA-256
`bbe9b9509a254c6c8c5057061635883c44dd0a02f106d1e61d3ac5d052dc3352`
drove all six `160x50` scenarios on Ubuntu 24.04.4/glibc 2.39 against Poky
5.2.4, BitBake 2.12.1, and qemux86-64. The verified evidence retains exact
ANSI, semantic text, cells/styles, reports, PNGs, manifest identity, and
checksums. Concept-to-live parity passes 6/6 with no open gaps.

```bash
YOCTUI_LIVE_COMPLETE=1 ./scripts/test-live-m22-concept-parity.sh
python3 scripts/test-m22-live-design-gallery.py
./scripts/verify-m22-concept-parity.sh
```
