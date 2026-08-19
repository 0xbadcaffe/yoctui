# Current Task

## Task

**ID:** COMPAT-LIVE-LATEST-001
**Title:** Validate latest supported Yocto stable release
**Status:** IN_PROGRESS

## Objective

Create current, independently verifiable live compatibility evidence from a
fresh official latest-supported stable Poky checkout. The evidence must bind
every claim to exact release and commit identities and must not be satisfiable
by deterministic fixtures.

## Dependencies

- `COMPAT-DOCTOR-001` — DONE
- `COMPAT-TEST-CMDS-001` — DONE
- `COMPAT-TEST-UI-001` — DONE
- `COMPAT-MATRIX-001` — DONE
- `COMPAT-BITBAKE-GETVAR-001` — DONE
- `COMPAT-DAEMON-RUNTIME-001` — DONE
- `COMPAT-PROBE-AGGREGATION-001` — DONE
- `COMPAT-BITBAKE-CANCEL-RUNTIME-001` — DONE

## Relevant files

- `scripts/verify-live-compatibility.sh`
- `docs/compatibility-evidence/`
- `docs/compatibility-matrix.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Evidence identifies the authoritative official release source, exact Poky,
  OE-Core, and BitBake commits, release/version, DISTRO, and MACHINE.
- Live daemon environment detection and capability probing agree with Doctor.
- Workspace discovery, Recipes, Layers, Configuration, one core build with
  native task/log events, Devtool capabilities, utility capabilities, and
  relevant modern BitBake commands are exercised against that environment.
- Results distinguish passed checks, unavailable capabilities, degraded
  behavior, and untested behavior without converting fixture evidence into a
  support claim.
- The repository evidence-age policy accepts the record and rejects stale,
  incomplete, synthetic, or version-ambiguous substitutes.

## Verification

```bash
./scripts/verify-live-compatibility.sh latest
./scripts/verify-roadmap.sh
```
