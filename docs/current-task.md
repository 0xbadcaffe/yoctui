# Current Task

## Task

**ID:** UX-DOC-001
**Title:** Document the polished one-stop workbench
**Status:** NOT_STARTED

## Objective

Complete the operator-facing and maintainer-facing documentation for the
polished workbench, its interaction model, authority boundaries, accessibility,
performance, dependencies, and verified live behavior.

## Dependencies

- `UX-LIVE-001` — DONE
- `UX-A11Y-001` — DONE
- `UX-PERF-001` — DONE
- `UX-PTY-E2E-001` — DONE

## Definition of done

- The operator guide, UI specification, architecture, keymap reference,
  terminal guide, settings, accessibility, and troubleshooting describe the
  implemented menus, focus, scrolling, widgets, workflows, and shortcuts.
- Rootfs/package composition documentation distinguishes manifest, pkgdata,
  deployed filesystem, partial, unavailable, and cleaned authority precisely.
- Dependency decisions, licenses, notices, SBOM, live evidence identities,
  validity windows, and reproduction commands remain complete and auditable.
- Screenshots and semantic captures correspond to the current production UI
  and are generated or validated by repository gates.

## Verification

```bash
./scripts/check-docs.sh
./scripts/verify-live-workbench-ux-evidence.sh
./scripts/verify-third-party-notices.sh
```
