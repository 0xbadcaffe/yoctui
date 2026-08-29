# Current Task

## Task

**ID:** UX-VISUAL-LIVE-001
**Title:** Recapture all six styled scenarios from the current release binary
**Status:** IN_PROGRESS

## Objective

Replace the historical monochrome/stale capture set with six current-commit,
color- and style-faithful screens after the M21 geometry remediation is reviewed
and installed.

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

The installed release and daemon currently share SHA-256
`d312172c3412eaf2b96c86a66c73ebddca7bc04fdac28d7f83df37e8c366a585`.
A real `160x50` bridge capture against `/home/bspguy-dev/src/build` verifies
Poky 5.2.4/qemux86-64 metadata, four layers, and numeric idle CPU, RAM, and
build-filesystem meters in the remediated Dashboard. The remaining gate is the
single-run six-scenario capture, not Dashboard composition or style fidelity.

```bash
YOCTUI_LIVE_COMPLETE=1 ./scripts/test-live-m22-concept-parity.sh
python3 scripts/test-m22-live-design-gallery.py
./scripts/verify-m22-concept-parity.sh
```
