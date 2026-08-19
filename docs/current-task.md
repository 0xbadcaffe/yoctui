# Current Task

## Task

**ID:** COMPAT-LIVE-OLDER-001
**Title:** Validate an older supported Yocto release
**Status:** IN_PROGRESS

## Objective

Validate Yoctui against one official, materially older maintained Yocto/Poky
release and record exact non-fixture evidence that safe baseline workflows are
preserved while newer or absent functionality degrades visibly and never emits
unsupported argv.

## Dependencies

- `COMPAT-DOCTOR-001` — DONE
- `COMPAT-TEST-CMDS-001` — DONE
- `COMPAT-TEST-UI-001` — DONE
- `COMPAT-MATRIX-001` — DONE

## Relevant files

- `scripts/verify-live-compatibility.sh`
- `docs/compatibility-evidence/older.toml`
- `docs/compatibility-matrix.md`
- `docs/compatibility.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Authoritative current Yocto documentation selects a genuinely older,
  maintained release with materially different BitBake/tool behavior.
- The official source and exact component/Poky commits, Yocto series/release,
  BitBake version, Yoctui commit, build identity, DISTRO, and MACHINE are
  recorded.
- Yoctui starts, identifies the environment, and preserves compatible Doctor,
  workspace, Recipes, Layers, Configuration, and core build/event workflows.
- At least one newer/absent behavior is disabled or limited with an exact
  capability reason, and no unsupported argv is spawned.
- The machine-readable record passes the evidence-age policy and cannot be
  satisfied by deterministic fixtures.

## Verification

```bash
./scripts/verify-live-compatibility.sh older
./scripts/verify-roadmap.sh
```
