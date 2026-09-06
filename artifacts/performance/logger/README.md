# tui-logger integration checkpoint

Version: 0.1.60. Compiler: rustc 1.97.0 (2d8144b78).

The existing pre-integration release matrix at commit `9dbbbecc` recorded
853,489 ns/frame for log-heavy output (500 frames, 160x48). This is a historical
comparison reference, not a newly measured 0.1.59 baseline.

The new adapter completed this development-profile workload:

```bash
YOCTUI_PROFILE_FRAMES=300 YOCTUI_PROFILE_SCENARIO=log-heavy \
  cargo bench --profile dev -p yoctui --bench workbench_profile --all-features
```

Observed: 300 frames, 6,674 ms, 22,248,286 ns/frame,
checksum `de610b277d5bd93d`. Debug timing must not be compared directly with
release timing or presented as satisfying the 10-ms release rendering gate
or the one-logical-CPU overhead target. The final M52 parent gate runs the
release matrix and independent CPU/responsiveness gates.

Measured development binary SHA-256:
`78dedda4397a7be23b271f89db2f10ec03d4d0af1845a5d6f07274e56f44e7da`.

Adapter source SHA-256:
`4e97d15b33d977d841390a2197bbdf10928b0e43d7bf29c726cc64db65d328e8`.

The complete UI suite passed before blessing version-only header updates.
Production cell/text fixtures changed only from 0.1.59 to 0.1.60; wrapping,
columns, selection and log semantics are unchanged. New tests independently
cover bounded projection, long Unicode payloads, word wrapping, concurrent
pane isolation, more than one hot-buffer batch, and no-color rendering.

## Release validation source and fixture

The initial v0.1.61 validation build uses immutable source checkpoint
`e3e7ecbd538c928d5d820b46ac26965026f2b064` plus the generated
[`release-source.patch`](release-source.patch), SHA-256
`d1f7909c4470084ebaa7e7bb865ad1986170c667a8b4cffbfe100a384070f36a`.
That patch changes package versions only; implementation sources are the
committed v0.1.60 sources. Captures identify the exact binary independently.

The real-Poky fixture is `/tmp/yoctui-m52-poky.nHLLjU`, initialized from the
same Poky 6.0.2 repositories with the OE-Core, meta-poky and meta-yocto-bsp
layers. Its local configuration selects qemux86-64/poky, eight BitBake workers,
`PARALLEL_MAKE = "-j 8"`, and `BB_NO_NETWORK = "1"`. It shares only downloaded
sources with the normal build; its sstate directory is private and the normal
build's sstate directory is a read-only file mirror. The mandated kernel
`cleansstate` therefore removes only fixture-owned outputs/cache entries.

Preparation exposed a pyenv shim recursion in the fixture's generated
`hosttools/python3`. The interrupted fixture task was stopped and its symlink
was replaced with `/usr/bin/python3`; subsequent commands run with the system
PATH before initializing Poky. No user Python configuration, Poky source
change, normal build output, shared-state cache, or installed daemon was
modified. Preparation then passed with a 100% sstate match and no downloads.

The frozen real-build executable is `/tmp/yoctui-m52-measured-release`, SHA-256
`a09faba6c1f1b795a18e62e560e19060f7d86792dde5572c9c036efdf94aa28b`.
Freezing the executable prevents concurrent Cargo benchmark builds from
replacing its pathname during a measurement. One independent idle-gate run
correctly rejected such an executable-identity change; the unchanged gate
passed after compilation finished.

## Pre-optimization release rendering and idle CPU

The complete 500-frame UI and 300-frame workbench matrices passed at 160x48
with the unchanged 10,000,000 ns/frame ceiling. Log-heavy measured 1,611,665
ns/frame; large-editor measured 9,523,294 ns/frame. The release benchmark
SHA-256 is `9cde6720d5d76f15739d06706449825b39c31cd1bd1b57c28100bbe8df763a8d`.
The current files in `artifacts/profile/` advance to the final candidate below.
An earlier contended matrix failed
large-editor at 11,891,508 ns/frame while optimized compilation was active;
[`rendering-contended.txt`](rendering-contended.txt) preserves that failure.

The dedicated log-heavy CPU profile covers 6,000 production frames at 499 Hz,
with frame-pointer call stacks, 5,025 samples and zero unresolved frames after
the existing bounded-quality filter. Its instrumented benchmark SHA-256 is
`fb37818e76af40bc6308097b38478eec6ac2f8f0b58a6f91b0fa33c967269b15`.
See [`log-heavy.svg`](log-heavy.svg) and
[`log-heavy-summary.txt`](log-heavy-summary.txt).

The initial idle suite measured 0.0623865% combined CPU using the documented
10-second warmup and sixty one-second samples. The independent current-release
gate then passed at 0.1663% combined. Both percentages are of **one logical
CPU**, not total machine capacity. Those earlier observations are historical;
the source-bound raw samples in `artifacts/performance/results/low-overhead/`
now describe the final optimized candidate below.

The initial real-Poky 120-second sample failed the hard 1% ceiling at 1.0169743%
combined. Host utilization was 99.742%; input p95 was 5.747 ms and all observed
IPC/cancellation/reconnect checks passed. The failed sample is retained in
[`real-poky-120-preliminary.json`](real-poky-120-preliminary.json); it is not
rounded down or presented as a passing release result. A longer 360-second
capture also failed at 1.0065159% combined, with 99.873% host utilization,
5.726 ms input p95 and successful IPC/cancellation/reconnect. Its raw evidence
is [`real-poky-360-preoptimization.json`](real-poky-360-preoptimization.json).
Neither failed capture satisfies the release CPU gate.

## Profile-guided fitting-ASCII path

The captured log-heavy self profile attributes 11.80% to grapheme iteration,
8.88% to span rendering and 7.81% to character counting. See
[`log-heavy-self.txt`](log-heavy-self.txt). The final validation candidate
therefore bypasses redundant grapheme work only for complete printable ASCII
lines that fit the width and remaining byte budget. It retains the prior
Unicode/control/overflow path and merges text/line/span styles identically.
No polling, input, retention, render cadence or performance threshold changes.

This candidate uses the same immutable checkpoint with
[`optimized-source.patch`](optimized-source.patch), SHA-256
`57334ee65a2b40f2ebaf3762bee02224912ae031471cc0ab49419652966d48eb`.
Adapter SHA-256:
`fe977125a094e6312a3d07cdb955b0985e45d407dbf633e921be986f86b660ea`.
The original version-only patch remains retained for the earlier captures.
The target test failed before implementation; all 273 UI tests (including
existing production goldens), strict workspace Clippy and focused checks pass.
The final optimized executable SHA-256 is
`2b7ff3659d2867e1463dc9a1f227624a879283142b4d3cbd7b6cd630e0dc6566`.
Its retained idle suite measured 0.0832617% combined CPU. The fresh real Poky
360-second capture passed at 0.9777483% combined on a 99.8601%-busy host,
with input p95 5.821 ms, cancellation acknowledgement 0.457 ms, fresh attach
26.586 ms and no disconnects. Maximum client queue depth was 83/256, with no
reliable waits, forced resynchronizations or slow-client disconnects.
Headroom below the strict 1% ceiling remains limited; prior failures are retained.

Both final quiet-host matrices passed: log-heavy 1,201,514 ns/frame,
large-graph 5,736,281 ns/frame and large-editor 8,685,776 ns/frame. The optimized
benchmark SHA-256 is
`bb88d1b0a264abb20a1056e99bd5ed4f61101b3c931e5e7fe0feae0ad13d9efb`.
One additional matrix during the real build failed large-graph at 18,460,704
ns/frame; those wall-time fixture thresholds are checked on the quiet host,
while saturation input/IPC responsiveness is checked separately.
Complete final matrices are retained in
[`next-generation-ui.txt`](next-generation-ui.txt) and
[`workbench-ux.txt`](workbench-ux.txt); the scripts also write their usual
transient copies under `artifacts/profile/`.

The real runtime profile (not a rendering fixture) sampled the same optimized
daemon/client for 60 seconds during kernel compilation: 275 samples, LBR at
499 Hz, 4,397 unresolved ppm within the existing real-build 15,000-ppm ceiling.
[`runtime.json`](runtime.json), [`runtime.svg`](runtime.svg) and
[`runtime.perf.txt`](runtime.perf.txt) retain identities, quality and stacks.
Leading self costs are JSON escaping (11.69%), memory movement (8.01%), terminal
cell styling (7.62%) and snapshot maintenance (4.73%); no unexpected polling
loop was identified. The profile is supplementary evidence, not a replacement
for the complete CPU sample series or the independent performance verifier.

`./scripts/verify-performance.sh` passed for the optimized candidate, including
the current-release idle CPU rerun (0.2080% combined), keyboard/frame/mouse
latency under saturation, IPC continuity and slow-client isolation, and the
current one-minute bounded-memory fixture. The historical 30-minute endurance
record is still checked against its unchanged source boundary; it is not a
new 30-minute run of this logger adapter. The complete workspace, strict
Clippy, ASan/LSan, documentation and roadmap checks also passed. Full repository
completion remains blocked by the strict Memcheck finding below.

## Strict Memcheck finding (release gate not passed)

`./scripts/valgrind.sh` failed on the v0.1.61 development-profile production
workload. An independent 16-frame run and the required 128-frame run each
reported exactly 39,367 possibly-lost bytes in ten allocations, zero definitely
lost bytes, and zero indirectly lost bytes. The unchanged sizes across an
eightfold increase in frames support fixed process-lifetime retention rather
than growth per frame; they do not override the strict gate.

[`memcheck-investigation.json`](memcheck-investigation.json) retains the exact
binary/source identities, raw-report hashes and allocation stacks. Of the
possibly-lost total, 34,995 bytes belong to upstream target-selection maps
(34,832 + 148 + 15), and 4,372 bytes belong to Jiff timezone data. The pinned
tui-logger source initializes its maps in the process-global `TUI_LOGGER`;
`set_buffer_depth` clears records but its public API does not destroy the
target maps or expose a complete logger shutdown. Jiff's timezone representation
uses tagged pointers. Valgrind documents that interior/tagged pointers can
produce this classification in its
[leak-detection manual](https://valgrind.org/docs/manual/mc-manual.html#mc-manual.leaks).

No suppression, accepted-leak category, threshold, or completion verifier has
been changed. A complete upstream cleanup/local-store API or an explicitly
reviewed, narrowly bounded policy exception is needed before this finding can
be accepted. ASan/LSan and bounded-projection tests passed independently; they
are not substitutes for the failed Memcheck gate.
The optimized 128-frame rerun reproduced the identical ten allocations and
39,367 bytes; see [`memcheck-optimized.json`](memcheck-optimized.json).
