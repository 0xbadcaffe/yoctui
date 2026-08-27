# Current Task

## Task

**ID:** UX-CONCEPT-ACCEPTANCE-001
**Title:** Make concept-screen acceptance scenario-complete and truthful
**Status:** IN_PROGRESS

## Objective

Replace anchor-only concept acceptance with explicit scenario feature and
evidence contracts. A scenario may pass only when its production-renderer
fixture, deterministic raster, and live navigation evidence each prove the
declared workflow, or when the missing capability remains assigned to an
incomplete registry task.

## Dependencies

- UX-CONCEPT-GOV-001 — DONE

## Definition of done

- Every concept scenario declares machine-checked required features.
- Fixture, raster, and live evidence are distinguished and validated.
- Missing scenario behavior remains an open gap owned by an incomplete task.
- Verifier failure tests reject false scenario attribution.

## Verification

```bash
python3 scripts/test-m21-concept-screen-verifier.py
./scripts/verify-m21-concept-screens.sh
./scripts/verify-roadmap.sh
```
