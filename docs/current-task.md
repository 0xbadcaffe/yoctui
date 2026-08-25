# Current Task

## Task

**ID:** README-UI-001
**Title:** Update README screenshots and UI documentation
**Status:** IN_PROGRESS

## Objective

Publish current redesigned-UI screenshots derived only from the canonical
real-Poky evidence bundle. Bind the README presentation to the tested Yoctui
binary, Poky revision, and evidence manifest rather than fixture-only media.

## Dependencies

- `LIVE-UI-POKY-001` — DONE
- `VISUAL-TEST-003` — DONE
- `PTY-UI-TEST-001` — DONE

## Relevant files

- `README.md`
- `docs/media/`
- `artifacts/release-quality/next-generation-ui/`
- `scripts/check-docs.sh`
- `scripts/verify-next-generation-ui-evidence.sh`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Replace fixture-only README presentation with current real-Poky UI views.
- Generate media deterministically from the canonical semantic captures.
- State the exact binary/source/Poky provenance represented by the media.
- Keep documentation links, media, and live evidence verification passing.

## Verification

```bash
./scripts/verify-next-generation-ui-evidence.sh
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
