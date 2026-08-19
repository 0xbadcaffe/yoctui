# Current Task

## Task

**ID:** COMPAT-CI-001
**Title:** Integrate compatibility tests into CI
**Status:** IN_PROGRESS

## Objective

Run deterministic capability compatibility gates in normal CI and provide a
bounded scheduled/manual live matrix that checks fresh official releases and
uploads diagnostics without imposing network-heavy Yocto work on every PR.

## Dependencies

- `COMPAT-TEST-CMDS-001` — DONE
- `COMPAT-TEST-UI-001` — DONE
- `COMPAT-LIVE-MATRIX-001` — DONE

## Relevant files

- `.github/workflows/ci.yml`
- `scripts/check-ci.sh`
- `scripts/test-release-compatibility.sh`
- `scripts/test-compatibility-matrix.sh`
- `scripts/verify-compatibility.sh`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Normal CI runs deterministic capability model, probe, command generation,
  dynamic UI gating, future-release behavior, and offline evidence checks.
- Scheduled/manual CI runs exact fresh older/latest official roles with bounded
  timeouts, without running on ordinary pull requests.
- Live failures upload Doctor, daemon, inventory, and BitBake smoke diagnostics.
- Workflow and helper validation rejects missing jobs, unsafe triggers,
  fixture-only live substitution, and omitted artifact publication.
- Local verification remains network-free unless explicitly opted in.

## Verification

```bash
./scripts/check-ci.sh
./scripts/test-release-compatibility.sh
./scripts/verify-roadmap.sh
```
