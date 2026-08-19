# Current Task

## Task

**ID:** COMPAT-DOC-001
**Title:** Document dynamic release correlation
**Status:** IN_PROGRESS

## Objective

Explain clearly that one Yoctui binary exposes functionality according to the
connected Yocto environment's detected capabilities, including release policy,
fallbacks, disabled reasons, future behavior, diagnostics, and live evidence.

## Dependencies

- `COMPAT-DOCTOR-001` — DONE
- `COMPAT-MATRIX-001` — DONE
- `COMPAT-LIVE-MATRIX-001` — DONE

## Relevant files

- `README.md`
- `docs/compatibility.md`
- `docs/compatibility-matrix.md`
- `docs/product-roadmap.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- User documentation states the Yocto-feature-correlated product semantics.
- Examples compare the same binary on older and newer environments, including
  unavailable Devtool behavior and a BitBake command implementation/fallback.
- Future unknown releases, unsupported/degraded releases, exact reasons, and
  the Environment/Compatibility inspector and Doctor output are documented.
- Supported/tested/expected/unknown claims remain distinct and link to exact
  current live evidence and renewal policy.
- Offline and opt-in live matrix commands are documented without implying that
  fixtures or optional development runs establish support.

## Verification

```bash
./scripts/check-docs.sh
./scripts/verify-compatibility.sh --structure-only
./scripts/verify-roadmap.sh
```
