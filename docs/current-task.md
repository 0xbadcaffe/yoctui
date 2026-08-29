# Current Task

## Task

**ID:** UX-LIVE-GALLERY-001
**Title:** Publish six real Yoctui screens as design regression baselines
**Status:** DONE

## Objective

Publish the six supported-host M22 Yoctui scenario screens under the design
documentation and make their exact provenance, membership, ordering,
dimensions, and bytes regression-tested.

M24 and `UX-LIVE-GALLERY-001` are complete. The next task is selected
from new user scope or a newly registered milestone; no required registry task
remains open.

## Dependencies

- UX-CONCEPT-LIVE-001 — DONE
- DEVWORK-001 — DONE

## Definition of done

- Six real supported-host Yoctui PNGs are directly visible under `docs/design`.
- A machine-readable manifest retains the exact live capture identity.
- Regression tests enforce exact scenario membership, order, README links,
  dimensions, hashes, and byte equality with the live evidence bundle.
- Documentation, M22 parity, and roadmap gates pass.

## Verification

```bash
python3 scripts/test-m22-live-design-gallery.py
./scripts/verify-m22-concept-parity.sh
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
