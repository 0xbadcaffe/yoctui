# Current Task

## Task

**ID:** COMPAT-MATRIX-001
**Title:** Define supported Yocto release matrix
**Status:** IN_PROGRESS

## Objective

Complete the release-policy matrix without promoting fixture, parser, or
partial live observations into support claims.

## Dependencies

- `COMPAT-SPEC-001` — DONE
- `COMPAT-OLD-001` — DONE
- `COMPAT-UNKNOWN-001` — DONE

## Relevant files

- `docs/compatibility-matrix.md`
- `docs/compatibility.md`
- `scripts/verify-compatibility.sh`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- The matrix distinguishes Claimed supported, Tested, Partially tested,
  Expected compatible, Unsupported, and Unknown.
- Every non-Unknown classification cites exact current evidence and scope;
  fixture-only or mocked tests never establish a live support claim.
- Minimum supported and latest supported releases remain unclaimed until their
  required live gates produce current policy-compliant evidence.
- Future/development and older environments retain capability-first behavior
  independently of matrix labels.
- Structure verification rejects ambiguous labels or unsupported claims.

## Verification

```bash
./scripts/check-docs.sh
./scripts/verify-compatibility.sh --structure-only
./scripts/verify-roadmap.sh
```
