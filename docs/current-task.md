# Current Task

## Task

**ID:** COMPAT-LIVE-LATEST-001
**Title:** Validate latest supported Yocto stable release
**Status:** IN_PROGRESS

## Objective

Produce current, non-fixture compatibility evidence from a fresh official
Poky checkout at the latest stable release selected from authoritative Yocto
release documentation.

## Dependencies

- `COMPAT-DOCTOR-001` — DONE
- `COMPAT-TEST-CMDS-001` — DONE
- `COMPAT-TEST-UI-001` — DONE
- `COMPAT-MATRIX-001` — DONE

## Relevant files

- `docs/compatibility.md`
- `docs/compatibility-matrix.md`
- `artifacts/release-quality/compatibility/`
- `scripts/verify-live-compatibility.sh`
- existing live Yocto/BitBake scripts
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- The selected release is the latest official stable release according to
  current authoritative Yocto documentation.
- A fresh official Poky checkout records exact release, Git commit, and
  BitBake version evidence.
- Live validation covers environment identity/probing, workspace discovery,
  one core build action with task/log events, Recipes, Layers, Configuration,
  Devtool capabilities, utility capabilities, and relevant modern commands.
- Evidence is bounded, machine-readable, non-fixture, current under repository
  policy, and independently accepted by the live compatibility verifier.

## Verification

```bash
./scripts/verify-live-compatibility.sh latest
./scripts/verify-roadmap.sh
```
