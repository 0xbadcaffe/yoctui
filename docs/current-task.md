# Current Task

## Task

**ID:** LOG-CONSOLE-IMAGE-001
**Title:** Complete Yocto log widget console and udev integration
**Status:** BLOCKED

## External blocker

The strict `./scripts/valgrind.sh` gate rejects 39,367 bytes in ten
`Leak_PossiblyLost` allocations from tui-logger global target maps and Jiff
timezone caches. Recorded sizes are unchanged across 16/128 production frames
and the optimized rerun; definite and indirect losses are zero. Upstream has
no complete public cleanup API. No suppression or relaxed threshold was added.

Unblocking requires upstream cleanup/local-store support or explicit approval
of a reviewed, narrowly bounded upstream-cache exception. The user was asked;
no approval has been received. Preserve the strict gate.
[Source-bound evidence](../artifacts/performance/logger/README.md).

## Dependencies

- IMAGE-UDEV-001 — DONE
- CONSOLE-TERM-002 — DONE
- YOCTO-LOGGER-ADAPTER-001 — DONE

## Verification

```bash
./scripts/valgrind.sh
./scripts/verify-performance.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

Refresh release evidence under the source-bound policy; historical v0.1.61
performance results do not automatically approve the v0.1.63 candidate.

## Completed independent work

M53 added the verified environment form and directory browser in v0.1.62.
M54 rewrote README, checked all six crate archives and installed optimized
v0.1.63 at the user's explicit request. Workspace/Clippy/formatting, 46 bridge
tests, documentation, package isolation and installed real-PTY startup passed.
The old executable and Cargo records are backed up, and the daemon was not
restarted. [Installation receipt](../artifacts/release-quality/cratesio/0.1.63.json).

RELEASE-PUBLISH-001 remains blocked by this task. No registry uploads or release
waivers occurred. No independent eligible tasks remain.
