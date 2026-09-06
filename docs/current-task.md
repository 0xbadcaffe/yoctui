# Current Task

## Task

**ID:** LOG-CONSOLE-IMAGE-001
**Title:** Complete Yocto log widget console and udev integration
**Status:** BLOCKED

## Objective

Resolve the upstream logger lifetime/strict Memcheck-policy conflict before
approving the integrated candidate. The logger evidence remains explicitly
bound to v0.1.61; v0.1.62 adds the independently verified M53 path browser. Do not mark this task DONE or
publish the candidate while the full completion gate remains unsatisfied.

## External blocker

`./scripts/valgrind.sh` rejects 39,367 bytes in ten `Leak_PossiblyLost`
allocations from tui-logger's global target maps and Jiff's timezone caches.
The exact sizes are unchanged across 16 and 128 production frames and after
the ASCII optimization; definite and indirect losses are zero. The pinned
upstream public API clears log records but cannot destroy its global target
maps. No suppression, unsafe private-state access or relaxed gate was added.

Unblocking requires upstream cleanup/local-store support or explicit approval
of a reviewed, narrowly bounded upstream-cache exception. The user was asked
about that policy choice; no approval has been received. Preserve the strict
gate until then. Reproduction, stacks, source/binary identities and prior failed
CPU measurements are in [the integration report](../artifacts/performance/logger/README.md).

## Prior v0.1.61 logger evidence

All 273 UI tests, workspace tests, strict Clippy, sanitizers and both quiet-host
rendering matrices passed. The optimized real Poky six-minute compile sample
measured 0.9777% combined CPU (one logical CPU), 5.821 ms input p95, healthy
cancellation/reconnect and no disconnects. Its source-bound raw evidence is
under `artifacts/performance/real-poky/`; the final idle suite is under
`artifacts/performance/results/low-overhead/`. SSH/QEMU coverage uses real-PTY
fixtures, not a live remote login or guest boot.
The aggregate `./scripts/verify-performance.sh` passed, including its fresh
idle, saturated input/IPC, slow-client isolation and bounded-memory checks.

## Dependencies

- IMAGE-UDEV-001 — DONE (v0.1.58)
- CONSOLE-TERM-002 — DONE (v0.1.59)
- YOCTO-LOGGER-ADAPTER-001 — DONE (v0.1.60)

## Definition of done

- Production fixtures are reviewed and current.
- Release CPU, latency, IPC, memory and real-Poky evidence validate.
- Workspace/Clippy/bridge/docs/completion checks pass.

## Verification

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-performance.sh
./scripts/valgrind.sh
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

M53 ENVIRONMENT-BROWSER-001 is complete in v0.1.62. Guided paths, directory
browsing, manual paste, safe cancellation and disconnected real-PTY startup
passed focused/workspace/Clippy/bridge/docs tests. The installed executable was
not replaced. Release evidence must be refreshed as required by source-bound
policy before approving a later candidate.

No independent later tasks are queued. The installed daemon and normal Poky
build are unchanged. The stopped, isolated `/tmp/yoctui-m52-poky.nHLLjU`
fixture is retained for reproduction (about 2.8 GiB); no shared cache was removed.
