# Current Task

## Task

**ID:** COMPAT-LIVE-MATRIX-001
**Title:** Add multi-release live compatibility validation
**Status:** IN_PROGRESS

## Objective

Create an opt-in, reproducible live compatibility matrix that selects
representative maintained official Yocto releases from authoritative policy
and validates exact evidence without making network-heavy work mandatory on
every local or pull-request run.

## Dependencies

- `COMPAT-LIVE-LATEST-001` — DONE
- `COMPAT-LIVE-OLDER-001` — DONE

## Relevant files

- `scripts/test-compatibility-matrix.sh`
- `scripts/verify-live-compatibility.sh`
- `docs/compatibility-evidence/`
- `docs/compatibility-matrix.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Representative maintained releases are selected from current official Yocto
  policy rather than a blind release-name list.
- The harness has an offline evidence-validation mode and an explicit opt-in
  fresh-source/live mode suitable for scheduled CI.
- Fresh runs use exact official source revisions, isolated build/runtime/state
  directories, bounded operations, and release-correlated diagnostics.
- Oldest proposed LTS, current stable, and optional development/snapshot roles
  are distinguished without converting optional or fixture results into claims.
- Latest and older evidence records pass together and disagree in the expected
  release/BitBake identities.

## Verification

```bash
./scripts/test-compatibility-matrix.sh
./scripts/verify-live-compatibility.sh --evidence-only
./scripts/verify-roadmap.sh
```
