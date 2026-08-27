# Current Task

## Task

**ID:** UX-LIVE-001
**Title:** Validate the one-stop workbench against supported live Yocto
**Status:** NOT_STARTED

## Objective

Validate the complete workbench against the supported older and latest live
Yocto environments and publish bounded, current release evidence.

## Dependencies

- `UX-WORKBENCH-CENTER-001` — DONE
- `UX-ROOTFS-UI-001` — DONE
- `UX-TERMINAL-UX-001` — DONE
- `UX-KEYMAP-E2E-001` — DONE

## Definition of done

- The supported older and latest Yocto/BitBake environments exercise menus and
  exact availability, a real build and cancellation, progress/log correlation,
  reconnect, and bounded large-data behavior.
- Image manifests, pkgdata, and filesystem composition remain correlated to the
  exact image/build identity with honest unavailable and partial states.
- Context terminals and supported interactive tasks such as devshell and
  menuconfig run through daemon-owned typed routes where the environment
  advertises them.
- Evidence records revisions, toolchain/environment identities, checksums,
  semantic PTY captures, outcomes, and timestamps and remains within the
  documented 90-day validity window.

## Verification

```bash
./scripts/test-live-workbench-ux.sh
./scripts/verify-live-workbench-ux-evidence.sh
./scripts/verify-compatibility.sh
```
