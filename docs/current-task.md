# Current Task

## Task

**ID:** UX-ROOTFS-ADAPTER-001
**Title:** Acquire bounded authoritative rootfs composition
**Status:** NOT_STARTED

## Objective

Acquire installed-package composition from the exact image manifest/pkgdata and
optional filesystem composition from the correlated BitBake-reported
`IMAGE_ROOTFS`, without escaping the active build or following symlinks.

## Dependencies

- `UX-ROOTFS-MODEL-001` — DONE

## Relevant files

- rootfs adapter and backend response/event wiring
- exact image manifest and generated pkgdata correlation
- optional `IMAGE_ROOTFS` acquisition and canonical containment
- no-follow traversal, hard-link deduplication, special-file accounting
- cancellation, stale denial, and count/depth/byte/time bounds

## Definition of done

- Manifest and pkgdata records match the selected image/machine identity; no
  filename guessing or stale generation is accepted.
- Filesystem scanning starts only from canonical `IMAGE_ROOTFS` contained by
  the active build, never follows symlinks, and deduplicates hard links.
- Regular files, directories, symlinks, and special files retain exact counts
  under entry, depth, byte, elapsed-time, and cancellation bounds.
- Missing/cleaned work state is unavailable; every bound or partial source is
  reported as an exact limitation without stale data.

## Verification

```bash
cargo test -p yoctui-bitbake ux_rootfs
python3 -m pytest bridge/tests -k rootfs
cargo test -p yoctui -- ux_rootfs
```
