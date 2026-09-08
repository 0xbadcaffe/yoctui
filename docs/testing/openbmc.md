# OpenBMC live integration

## Installed package mapping repair (v0.1.85)

The exact live libblkid1 regression first failed because installed-package
authority was Partial. The adapter now resolves generated runtime-reverse
links only in the single-hop ../runtime/<original-package> form, with contained
non-symlink directories and regular runtime records. PKG corroborates renamed
installed identity; original names scope PKGSIZE/FILES_INFO. Missing indexes
retain direct-name compatibility, but invalid mappings do not silently fall
back. Four new cases cover installed identity/deduplication, unsafe and
conflicting mappings, byte limits, partial results, cancellation and deadline.
All 13 focused rootfs tests and 281 UI tests pass. All 1584 workspace tests and
doc-tests, 52 bridge tests, version-policy tests/checks and roadmap pass.
Strict Clippy, production build, fmt and documentation also pass; the task
is DONE and OPENBMC-ROOTFS-SOURCES-001 becomes current.
Seventeen golden diffs are version digits only; six rasters verify unchanged
apart from that version label.

The [actual v0.1.85 package screen](../../artifacts/live-openbmc/romulus/v85-image-packages-20260908.txt)
now reports Installed packages: available. All 228 installed packages have
metadata: 81062873 installed bytes and 1778 files, versus the earlier partial
56717557 bytes and 1668 files. All 40 missing-pkgdata warnings are gone. Overall
composition remains Partial solely because the separate IMAGE_ROOTFS source
query is not connected; that finding is OPENBMC-ROOTFS-SOURCES-001, not waived.
The first connected frame arrived in 0.369 seconds; this single read-only
capture is not a release-performance claim. No daemon restart or image rebuild
occurred. Exact captured/workspace-tested binary:
/home/bspguy-dev/.local/state/yoctui-v85-validated.YhRRZA/yoctui, SHA-256
cf19348c978d8e3810874d49cad4875aa5a1194bce0dd21871de1699f32bd9b0.
Private daemon remains v0.1.84; installed release stays v0.1.64.

## Successful image and inspection findings (v0.1.84)

The fifth obmc-phosphor-image attempt completed naturally through the private
v0.1.76 Yoctui daemon: 6812/6812, job 1 Exited exit 0, successful typed Completed
event at sequence 49999 and terminal snapshot 50002. The old daemon stopped
normally only after success. See the [terminal record](../../artifacts/live-openbmc/romulus/image-terminal-v76-20260908.json),
[completed production screen](../../artifacts/live-openbmc/romulus/v84-image-completed-20260908.txt)
and [artifact report](../../artifacts/live-openbmc/romulus/image-artifacts-20260908.json).
Source is clean OpenBMC d4fd7d3f54e88e800c0284b753af68a13aabbef6, MACHINE romulus,
DISTRO openbmc-openpower, BitBake 2.19.0. All Poky data is preserved.

The deployed obmc-phosphor-image-romulus-20260908040925.static.mtd is 33554432
bytes, SHA-256 00c37b3aed9961a37d50a70afc3c670c50a25282c93784f7022b8e67d1c4f7f0.
Its SquashFS-XZ is 23498752 bytes, SHA-256
ebe8cca00405777d532bb8bc5573f30e7d3703c9f7758fcf2e7bdcf4a4a6cefe.
unsquashfs verifies the filesystem, and archived os-release and bmcweb bytes
match the retained rootfs. The manifest contains 228 packages; rootfs contains
274 service files. bmcweb is a stripped ARM EABI5 ELF; its generated IPK has
readable control metadata. Kernel, Romulus DTB, U-Boot and six-package initramfs
are also deployed. These are build/artifact checks, not a firmware boot.

One warning, zero errors: Group render has never been defined. The source's
rootfs-postcommands.bbclass systemd_sysusers_check compares declarations with
passwd/group. usr/lib/sysusers.d/basic.conf declares render but etc/group lacks
it. This is investigated upstream image metadata, not a Yoctui build/protocol
failure; nothing was patched or suppressed and runtime impact is untested.

The exact workspace-tested v0.1.84 candidate is preserved at
/home/bspguy-dev/.local/state/yoctui-v84-validated.Gh0uoK/yoctui, SHA-256
c1fd3f3ccd2c269d3c7d11d35e1f7b69deb1a6c5ff0a553b33621c0aa9f0d221.
Its private daemon PID 2310652, instance 4832e5d8285444a5445264dae2900239,
loaded 4799 recipes and recovered jobs 1/2 unchanged. The real two-target
llvm-native/stdplus listtasks recheck used new job 3, exited 0 and completed
2/2. Exact PNs correlate queue/start/completion without ghost rows; observed
timestamps survive terminal compaction. [Raw lifecycle evidence](../../artifacts/live-openbmc/romulus/v84-lifecycle-recheck-retry-20260908.json)
retains the explicit stale-generation rejection and bounded safe refreshed
submission; the initial harness rejection submitted no job. The actual build
interval was 1788858340539 to 1788858342353, 1814 ms. Both
[first](../../artifacts/live-openbmc/romulus/v84-observed-terminal-first-20260908.txt)
and [second](../../artifacts/live-openbmc/romulus/v84-observed-terminal-second-20260908.txt)
fresh production attachments show 00:00:01, Workers 0, Active 0 and Waiting 0.
The isolated two-attachment 64-second fixture also passes.

The [Images screen](../../artifacts/live-openbmc/romulus/v84-images-20260908.txt)
lists 21 artifacts. Selecting the actual image manifest exposes two reproducible
inspection gaps, registered as separate tasks before implementation:

- OPENBMC-PKGDATA-001: [packages](../../artifacts/live-openbmc/romulus/v84-image-packages-20260908.txt)
  lists all 228 installed names but 40 metadata records are unavailable.
  pkgdata/runtime-reverse/libblkid1 points to ../runtime/util-linux-libblkid;
  the scanner only tries runtime/libblkid1. Resolve contained generated name
  mappings and original scoped keys without relaxing general symlink safety.
- OPENBMC-ROOTFS-SOURCES-001: [filesystem](../../artifacts/live-openbmc/romulus/v84-image-rootfs-unavailable-20260908.txt)
  reports IMAGE_ROOTFS was not reported although the retained rootfs exists.
  Attached clients instantiate ProcessBackend whose get_variable returns None;
  the existing bridge get_rootfs_sources is not connected to this path.
  Acquire exact image sources through daemon-owned metadata authority, without
  guessing workdir paths or requiring an initialized local attach environment.

OPENBMC-LIVE-001 is not DONE until both fixes and actual UI rechecks pass.
v0.1.84 registration changes no runtime code; baseline passes 1580 workspace
tests/doc-tests, 52 bridge tests, strict Clippy, fmt, docs, rasters, roadmap
and version policy. All 17 golden diffs are version digits only. The normal
installed release stays v0.1.64. Full completion still requires fresh
source-bound real-Poky performance evidence and the remaining release gates.

Earlier sections below are historical observations at their named versions;
pending-image statements there are superseded by this successful result.

## Active-worker repair (v0.1.83)

OPENBMC-WORKER-COUNT-001 is DONE. Eight focused tests, all 281 UI tests and all
1,580 workspace tests including doc-tests pass, together with 52 bridge tests,
strict Clippy, fmt, docs, rasters and roadmap. The preserved binary also passes
the isolated terminal-timing reattachment check. The first
test needed its imports and exact 160x50 context-header
geometry corrected, then reproduced Workers: 0 instead of 2. The model now
counts complete positive active PIDs, otherwise complete nonblank active labels,
with deduplication inside a single namespace. Current replica/build authority
is mandatory. Missing/partial identity and lost authority remain unavailable;
current idle/terminal builds report zero. Aggregate task totals are not worker
counts. Five model cases, one live/batched/snapshot/reconnect app integration
and two responsive UI cases cover normal and missing-authority paths.

Reviewed golden changes beyond version digits are confined to the Workers
field: terminal 1 becomes 0 in the failed concept and failed target fixtures;
reconnecting 1 becomes unavailable. All other symbols/styles/layout remain
unchanged. Six deterministic rasters were refreshed and verified; the failed
build raster was visually reviewed. The real image daemon remains untouched.

The [real v0.1.83 capture](../../artifacts/live-openbmc/romulus/v83-worker-count-20260908.txt)
now shows Workers: 2 and Active 2 at 5936/6812. A read-only typed snapshot
confirms rust-native do_install PID 1618606 and qemu-system-native do_compile
PID 2077511, both without worker labels. The selected QEMU task reports 59%
progress. The first connected frame arrived in 4.873 seconds; legacy timing
remains unavailable. This validates the client-only worker projection against
the unchanged v0.1.76 daemon, not the undeployed queue/timing bridge repairs.

The captured and workspace-tested binary is preserved at
/home/bspguy-dev/.local/state/yoctui-v83-validated.O7H2jM/yoctui, SHA-256
041e10543ec703d66031b8803e4809e33a0339cf7222d3ae9c63d5b868ce5713.
No build was submitted, cancelled or restarted for this read-only check.

## Active-worker header finding (v0.1.82 registration)

The [v0.1.81 production capture](../../artifacts/live-openbmc/romulus/v81-active-image-20260908.txt)
shows 5083/6812 and Active 2, but Workers: 0. The rust-native task has PID
1618606 and no worker label; tar configuration is also active. The header's
HashSet counts optional labels across all retained task states, so absent
labels become zero and completed labels can inflate the result. This is an
observed display defect, not evidence that BitBake stopped executing workers.
OPENBMC-WORKER-COUNT-001 registers a typed authoritative active-worker repair;
no implementation is claimed by the v0.1.82 governance change.
Registration checks pass: 279 UI tests, formatting, docs, roadmap and version
policy checks/tests; 17 version-only goldens and six refreshed rasters. No
runtime source changed, so no full workspace or release-gate claim is made.

The capture's first connected frame arrived in 2.669 seconds. Counts agree
with the observer and legacy timing remains unavailable. Ghost git queue rows
remain expected from the unchanged v0.1.76 bridge. The image has no terminal
result. BusyBox 1.38.0-r0 is present as an arm1176jzs IPK with readable control
metadata; package production alone does not establish a complete image/rootfs.

## Queue identity repair (v0.1.81)

OPENBMC-TASK-IDENTITY-001 is DONE. The failing bridge regression returned
llvm_git, vendor_git and not-the-pn for authoritative llvm-native,
lib32-actual-name and overridden-name metadata. The lookup now uses exact
getRecipes PN/file pairs after BuildStarted, once per build, preserving virtual
native/multilib paths. The local BitBake 2.19 source marks getRecipes read-only
and fires BuildStarted after buildTaskData has initialized its recipe cache.
No queue event parses metadata or derives PN from a filename. The lookup uses
handle_events=False: Tinfoil's default command cleanup otherwise drains and
discards native events. Tests require this flag and retain all queue/start/end
events; updateCacheSync returns immediately for the initialized running cooker.

Three new bridge tests cover metadata variants/overrides, limits and conflicts,
per-build refresh, duplicate starts, lookup failure and unresolved statistics.
All 52 bridge tests and six new Rust protocol/backend/model/app/CLI/TestBackend
regressions pass. Unresolved identity carries aggregate-only task_stats;
daemon publication reuses existing running BitBake job progress, without adding
an incompatible daemon event tag. Only a current started-build aggregate takes
those counters. The bounded lookup uses the initialized default-multiconfig
cache; absent or nonmatching keys remain unresolved rather than guessed.

This is fixture and local-source evidence. The real image continues on the
preserved v0.1.76 daemon; validate the new bridge after a natural stopping point,
without interrupting the image solely to deploy it.

Python linting/type checks pass and bridge coverage is 78.35% (required 75%).
The final full Rust run passes all 1,572 tests and doc-tests, including the
event-drain flag and lost-authority guard. Strict Clippy, formatting, docs,
six rasters and roadmap checks pass. Cargo checks now run
sequentially: one overlapping default-profile docs build caused workspace
doc-tests to fail resolving a dependency artifact while it was being replaced.
The sequential rerun passed without weakening tests. The verified executable
is /home/bspguy-dev/.local/state/yoctui-v81-validated.18e7ox/yoctui, SHA-256
0dcce028d77c7218d508914e1a3b02fc7828ffb7ed05b405b70bba8caaebd364.
It has not replaced the running image daemon or normal installed release.

## Reattachment timing repair (v0.1.80)

OPENBMC-ATTACH-TIMING-001 is DONE with optional daemon-observed lifecycle
timestamps. Ten focused tests and all 1,566 workspace tests pass, together with
49 bridge tests, strict Clippy, fmt, docs, rasters and roadmap. The legacy app regression first failed because
replay produced a local SystemTime instead of None, then passed after explicit
observed-time reducer inputs were added. Protocol regressions cover completion
compaction, bounded eviction, duplicate terminal timestamps and legacy wire
decoding. App/UI tests inject observation times rather than sleeping to infer
durations.

`python3 scripts/test-snapshot-timing.py <candidate-binary>` uses a private
temporary socket and the actual production PTY client, without touching the
OpenBMC daemon. Its injected terminal snapshot starts at Unix millisecond 1000
and ends at 65000. The preserved v0.1.78 binary reproducibly displayed 00:00:00,
not 00:01:04. The regression requires two fresh clients to retain the latter
duration across subsequent frames. This is isolated client integration evidence,
not a real BitBake lifecycle-timestamp validation. The running image stays on
v0.1.76; final live timing validation remains under OPENBMC-LIVE-001 after a
natural stopping point.

The first v0.1.80 production check exposed a second path: the header directly
subtracted the build start from the current clock, bypassing the terminal-aware
model summary. It displayed 496901 hours for the injected epoch timestamp.
The header now consumes the same summary as the other views; the rendering
test also checks the header's explicit Elapsed label. The initial full workspace
run stopped on this integration regression; the final full rerun passes.

The corrected production client passes both isolated attachments. A read-only
[v0.1.80 client capture](../../artifacts/live-openbmc/romulus/v80-legacy-timing-20260908.txt)
against the still-running v0.1.76 OpenBMC daemon shows 3842/6812, task/build
elapsed unavailable and Started unavailable, rather than newly invented
five-second clocks. net-snmp and rust-native installation tasks are active;
this does not claim that the older daemon now publishes timing. The binary hash is recorded in the
[capture metadata](../../artifacts/live-openbmc/romulus/v80-legacy-timing-20260908.json).
No image job was submitted, cancelled or restarted by this check.

Verification also corrected two test issues without relaxing timing authority:
the UI assertion now distinguishes current-header timing from valid retained
history, and the private socket fixture tolerates a peer closing before its
post-capture detach acknowledgement. Snapshot delivery and both screen-duration
checks remain mandatory. Three consecutive standalone production-client checks
pass after the cleanup-race correction.

Status: environment, direct-capability, startup-responsiveness, bounded
inventory, CLI submission, symlinked tool discovery, host Python and initial
attachment tasks complete. The third Romulus image attempt was accepted through
the preserved Yoctui v0.1.73 candidate after host memory recovery on September 8.
No completed image is claimed. See the current recovery record below.

## Planned machine and isolation

Use `romulus` and `obmc-phosphor-image`. The
[OpenBMC development guide](https://github.com/openbmc/docs/blob/master/development/dev-environment.md)
uses Romulus for its CI-tested example. OpenBMC is a Yocto-based distribution,
not a prebuilt SDK. The
[upstream README](https://github.com/openbmc/openbmc/blob/master/README.md)
documents sourcing `setup` from the repository root with a machine and optional
build directory.

Isolated paths (both were absent before setup):

- Source: `/home/bspguy-dev/src/openbmc`
- Build: `/home/bspguy-dev/src/build-openbmc-romulus`

Cloned upstream with `git clone --depth 1` at commit
`d4fd7d3f54e88e800c0284b753af68a13aabbef6` (197 MiB checkout). Initialized with
`. setup romulus /home/bspguy-dev/src/build-openbmc-romulus` from its source root.
Host: Ubuntu 26.04, eight logical CPUs, approximately 15 GiB RAM. Required
compiler/archive tools and `unshare -Ur true` preflight pass; no sysctl changed.
Default build parallelism is unchanged.

Live identity: BitBake 2.19.0, MACHINE romulus, DISTRO openbmc-openpower,
OE-Core series blacksail/wrynose, nine configured layers. Yoctui daemon
v0.1.65 uses private XDG config/state directories under
`/home/bspguy-dev/.local/state/yoctui-openbmc-validation` and runtime directory
`/run/user/1000/yoctui-openbmc-validation`. Its socket is in `yoctui/daemon.sock`
under that runtime directory. It does not own the Poky workspace.

The [doctor capture](../../artifacts/live-openbmc/romulus/doctor-v0.1.65.txt)
has SHA-256 `0e138dcbf10e62725e0e5fb699dca7b26de50af3df285c2a164feaf1884789d9`.
It confirms Current authority and a bounded bridge handshake, not full release
support or a completed build. The debug candidate's hash is recorded in
[compact telemetry evidence](compact-telemetry.md#delivery-boundary).

Environment-task verification: 276 UI tests pass after the v0.1.66 governance
version bump; all 17 cell/text fixture diffs are version-only. Six rasters were
refreshed, and documentation and roadmap checks pass. Runtime investigation
continues with the pinned v0.1.65 daemon until its replacement is verified.

## Storage dependency

On 2026-09-07, root had only 3.9 GiB available before verification compilation.
Poky's `build/tmp` occupied 67 GiB, its sstate-cache 9.0 GiB and downloads 8.1 GiB.
No other persistent filesystem with build capacity was mounted. The upstream
guide recommends a 200 GB disk for its development VM; this is guidance, not a
guaranteed peak for the selected revision. A few GiB free is not a safe budget.

Only the agent-generated Yoctui address/leak sanitizer compiler caches were
cleaned with `cargo clean --target-dir` (9.9 GiB reported by Cargo). They can be
regenerated by the sanitizer verification script. No Poky output was removed.
The first full Rust workspace test compilation failed with ENOSPC before that
cleanup; retry is required and recorded independently of any test pass.

Resolution: after all workspace, Clippy, bridge and docs checks passed, saved
the tested candidate executable, then ran `cargo clean --workspace --profile dev`.
Cargo removed 79.1 GiB of regenerable debug artifacts, leaving 78 GiB free.
The installed release, sources and all Poky data remain untouched. Rebuilding
the Rust artifacts remains possible from the committed source. No permission
to delete Poky output is needed for this route.

The new OpenBMC local.conf sets STOPTASKS at 15 GiB and HALT at 8 GiB for
TMPDIR, DL_DIR and SSTATE_DIR; /tmp limits are 1 GiB and 500 MiB respectively.
Monitor usage; 78 GiB is an initial budget, not a guarantee of full completion.
If it proves insufficient, stop safely and request additional storage before
removing any user build output.

## Observed integration defects

- OPENBMC-CAPABILITY-001: daemon startup always passes absent backend probe
  evidence. BitBake 2.19 therefore cannot resolve build/API capabilities beyond
  the documented 2.18 version fallback. Fix direct probing with bounded real
  API evidence, not a permissive version assumption.
  Fixed in v0.1.67: the [one-shot probe](../../artifacts/live-openbmc/romulus/backend-probe-v0.1.67.json)
  reports 13 catalog API tokens. The [daemon JSON report](../../artifacts/live-openbmc/romulus/doctor-v0.1.67.json)
  confirms build, cancellation and native events are Available through direct
  backend negotiation. The fallback version range was not widened. Invalid
  reports remain inconclusive; the first integration attempt correctly rejected
  an extra noncatalog `getvar` token, which was removed from the probe.
- OPENBMC-CLI-BUILD-001: two invocations of
  `yoctui daemon build obmc-phosphor-image` rejected StaleGeneration (current
  generation 5 then 6) despite fresh attaches. Both exited zero. No build was
  accepted. Test snapshot/attach ordering and rejected-command exit status.
- OPENBMC-STARTUP-001: after direct probing enabled the metadata path, daemon
  startup blocked on `list_recipes`/`parseFiles` and exceeded its 180-second
  readiness deadline. The foreground daemon and Cooker/parser children kept
  running; status reported an absent runtime record. Fix metadata scheduling
  and startup-timeout lifecycle ownership before the image build.
- OPENBMC-TOOLS-001: executable discovery rejects OpenBMC's symlinked `scripts`
  PATH directory. Both lexical and canonical Devtool help invocations work, but
  daemon evidence says the executable is absent. Canonicalize the directory
  while retaining existing per-file alias checks.

Capability-fix verification: 1,523 workspace tests (4 existing ignored), 47
bridge tests, strict Clippy, formatting, Ruff/mypy and documentation checks pass.
Focused coverage includes 12 daemon compatibility and 66 backend compatibility
tests. The startup scan eventually finished and the private socket served
Current authority, but this does not resolve the 180-second lifecycle defect.
The live probe hash is
`ece1d80f04608b795cb47b49288e25b5509dda3fa87c2e4638f330e2c3218bc2`;
daemon JSON hash is
`313528101babdad46f71dd366cee3cd4f40b7058ab8067ca34e05e35a3b8b540`.

## Required live evidence

### Startup responsiveness candidate v0.1.68

The [startup capture](../../artifacts/live-openbmc/romulus/startup-v0.1.68.json)
records readiness at 86.16 seconds, while metadata was still loading. Attach
took 26.86 ms; stopping during that scan took 5.08 seconds. Afterwards no owned
daemon, bridge, Cooker or parser process remained. The isolated daemon was
restarted (70.68 seconds) for the
[inventory capture](../../artifacts/live-openbmc/romulus/inventory-v0.1.68.json).
The client remained attached through real parsing, receiving 26 events over
123.73 seconds. The scan then reported a 1 MiB bridge frame overflow, not a
disconnect. This is required follow-up OPENBMC-INVENTORY-001, not inventory or
image-build success. The capture records the running executable's hash, rather
than the subsequently rebuilt binary at the same disk path.

Startup uses one owned result slot, a ten-minute inventory deadline and bounded
interrupt/reap. Full inventory does not gate IPC readiness; build commands get
an explicit metadata-loading conflict while the scan owns the connection.
Failed parent startup reaps only its own unreaped foreground child. Tests cover
slow-scan cancellation, failed-worker completion, process reaping, Python cleanup
and deferred Workspace publication to an already attached client. The six
production rasters and 17 cell/text fixtures change only version digits.
Final validation: 1,530 workspace tests (four existing ignored), 47 bridge tests,
strict Clippy, formatting and documentation checks pass. The cleanup test's
readiness marker was moved inside its Python try/finally to remove a test-only
race between advertising readiness and installing the cleanup scope.

### Bounded inventory candidate v0.1.69

The [size measurement](../../artifacts/live-openbmc/romulus/inventory-size.json)
found 4,799 recipes serialized into 1,051,217 bytes (largest record 300 bytes).
The [live transfer capture](../../artifacts/live-openbmc/romulus/inventory-v0.1.69.json)
shows the complete 4,799-recipe, nine-layer Workspace arriving 20.48 seconds
after attach, without reattachment. Its running-binary SHA-256 is recorded in
the capture. Startup took 141.43 seconds while Rust checks also ran; attach took
294.69 ms. These are one-shot startup observations, not p95 performance claims.

Opt-in chunks are capped at 512 KiB, the total at 16,384 records / 3 MiB. Request
correlation, contiguous offsets, stable totals and explicit completion are
mandatory; incomplete data never becomes authoritative. Legacy callers retain
single-frame responses when they fit and otherwise get an explicit limit error.
The 1 MiB bridge and 4 MiB daemon frame limits are unchanged. A daemon snapshot
overflow reports a scan error and keeps IPC usable.

Validation: 1,535 workspace tests (four existing ignored), 49 bridge tests,
strict Clippy, formatting, Ruff/mypy and docs pass. The first broad run hit
ETXTBUSY in an unrelated pkgdata process fixture; its isolated test and the full
rerun passed. All 17 cell/text fixture changes are version-only; six rasters
were regenerated. The measurement also recorded an upstream os-release warning
because this shallow checkout has no reachable Git tag; inspect this during
image-build validation rather than hiding it.

### CLI submission candidate v0.1.70

Five production-CLI regression cases failed before the change. Six socket-backed
test groups now cover stale refresh/retry, exhaustion, rejection exit status,
wrong request IDs, changed workspace, ambiguous I/O, protocol errors, bounded
event flooding, pings, and accepted-build cleanup disconnects. The CLI permits
three total submissions only after explicit stale rejection and an unchanged
daemon/workspace/compatibility snapshot identity; other failure paths never
automatically resubmit.

The [live rejection capture](../../artifacts/live-openbmc/romulus/cli-rejection-v0.1.70.json)
records v0.1.70 talking to the idle v0.1.69 OpenBMC daemon. `invalid/target` is
rejected by BuildRequest validation before a job can be allocated. The first
attempt encountered stale generation; after refresh, attempt 2/3 reported the
validation error with exit code 1. The daemon retained zero jobs. This validates
the real CLI dispatch/error path, not image-build acceptance.

All 1,541 workspace tests (four existing ignored), 49 bridge tests, strict
Clippy, formatting, Ruff/mypy and documentation checks pass. Fixture/raster
changes are version-only. Actual obmc-phosphor-image submission follows the
symlinked-tool discovery fix.

### Canonical tool discovery candidate v0.1.71

The directory-symlink regression failed before the fix. Fourteen focused daemon
compatibility tests now pass, including basename-sensitive sibling aliases and
rejection of relative, dangling, escaping and nonexecutable candidates. Absolute
initialized PATH directories are canonicalized; the existing final-file alias
safety checks remain unchanged.

The [live doctor report](../../artifacts/live-openbmc/romulus/doctor-v0.1.71.json)
finds Devtool, Recipetool and oe-pkgdata-util at their canonical OpenBMC paths.
The package-list command capability is Available. Devtool status and some
Recipetool operations remain Unknown because their bounded help probes timed
out, not because the executables are missing. No unsupported operation was
enabled from executable presence alone. Generated package data is correctly
unavailable before any image/package build.

The private daemon started in 113.21 seconds with running executable SHA-256
`031841516c11bf8ef9d39cda7414737cd169d68b44b30b58d00bfa9589b61b43`.
The JSON report SHA-256 is
`91e4d008820ad7323f114dd0e85a7e9d12225c433f0c80c9fc68db8f5fd3c248`.
All 1,543 workspace tests (four existing ignored), 49 bridge tests, strict
Clippy, formatting, Ruff/mypy and documentation checks pass. Version-only
goldens and six production rasters were refreshed. Another 58.3 GiB of agent
Cargo debug outputs were cleaned before rebuilding; no Poky data was removed.

### Initial attached-client startup candidate v0.1.73

A socket fixture delayed Attached by 400 ms and failed with the old 250 ms
read timeout. It now passes with a five-second pre-terminal read budget,
installing the daemon's build directory in a client with no local environment.
A second regression found that using the same timeout for socket discovery
delayed disconnected startup by five seconds; socket discovery is now separately
capped at 250 ms. The interactive reconnect timeout remains unchanged.

The [real attached terminal](../../artifacts/live-openbmc/romulus/initial-attach-fixed.txt)
opens directly on Dashboard with no navigation before capture and with BUILDDIR,
YOCTUI_BUILD_DIR and PYTHONPATH removed. It shows the daemon's OpenBMC identity
and compact resource meters, not local environment setup. The
[capture record](../../artifacts/live-openbmc/romulus/initial-attach-fixed.json)
records the running client binary hash and 1.336 seconds to the first connected
frame at 140x32. This one-shot startup observation is not an input-latency
percentile or steady-state CPU result.

After the persistent Python repair, the
[daemon doctor report](../../artifacts/live-openbmc/romulus/doctor-system-python.json)
records 84 Available capabilities, including Devtool status/modify and
Recipetool create. The private daemon's running executable hash is
`1240dc7e83c8f0bf5bcdfac282d6013539a42fdfa384687b6266c4e4e86f8b63`;
report hash is `7d42406e1d787ca0d051e8edf44efd41d0b93b837be61d89c701fc991b2b3e70`.
It is the v0.1.72 candidate and remains isolated from the normal user daemon.

All 1,545 workspace tests (four existing ignored), 49 bridge tests, strict
Clippy, formatting, Python and documentation checks pass. The 17 cell/text
fixtures changed only version digits; six production rasters were regenerated.

### Image retry acceptance

The new workspace enables rm_work for completed package work directories,
excluding obmc-phosphor-image so rootfs inspection remains possible. All Poky
data and default parallelism remain unchanged. Fetching upstream Git history
and tags (without changing HEAD) makes git describe report
3.1.0-dev-1106-gd4fd7d3f54 and resolves the shallow-checkout os-release warning.
The first premature build submission was explicitly rejected while initial
inventory loaded. After Workspace readiness, the CLI accepted image job 1.

Live compact capture at 100x24 shows CPU/RAM/FS meters. A client without a local
environment initially opened environment setup despite the active daemon:
the 250 ms initial snapshot timeout is too short. A read-only attach observation
took 767 ms including detach (not an isolated latency percentile). This is
OPENBMC-ATTACH-001. The first capture also demonstrated that the older generic
capture script starts a local bridge unless the environment is sourced; the
subsequent captures explicitly use yoctui attach and are not fixture renders.

### September 8 host memory recovery

The [second-attempt observation](../../artifacts/live-openbmc/romulus/image-build-retry-073.json)
ends at 4,953/6,812 tasks with `terminal: null`. At resumption, daemon PID
3955092, its runtime socket and all build workers were absent; no Romulus
image deployment directory existed. The host did not reboot. Kernel journal
entries record global OOM kills at 03:50:32–33 local time, followed by the user
manager shutting down at 03:50:42. These prove host memory exhaustion and
session loss, not which process caused the exhaustion or an image-build result.
Reproduce the relevant read-only checks with:

```sh
journalctl -k --since '2026-09-08 03:00:00' --until '2026-09-08 03:55:00' \
  --no-pager -g 'Out of memory: Killed process'
journalctl --user --since '2026-09-08 03:50:40' --until '2026-09-08 03:50:45' \
  --no-pager
```

The old durable daemon checkpoint predates that second attempt. Its historical
Failed job must not be interpreted as the terminal result of the interrupted
retry. Job numbers can repeat across daemon instances; correlate every capture
with its recorded instance and executable hash.

The preserved candidate `/tmp/yoctui-v73-validated.vMBk6H/yoctui` reports
v0.1.73 and SHA-256
`f34b9b26fb6d60d1503b20674bf194252552159ca63953b37000eb3e0b817a26`.
An initial recovery daemon was stopped while idle, then only
`/home/bspguy-dev/src/build-openbmc-romulus/conf/local.conf` gained:

```bitbake
# Bound this validation build after the host OOM on 2026-09-08.
BB_NUMBER_THREADS = "2"
PARALLEL_MAKE = "-j 2"
```

Both values were verified through `bitbake-getvar --value` before submission.
No source revision, Poky data, swap configuration or system service was changed.
Removing these two assignments restores the original concurrency defaults;
do that only after stopping the associated build. Existing rm_work exclusions
and disk margins remain in force. Root had about 46 GiB free and the host about
9.6 GiB available RAM at recovery preflight; these are observations, not resource
guarantees.

The private daemon restarted from the initialized OpenBMC setup environment
with the same XDG paths documented above. The third attempt was submitted using
`yoctui daemon build obmc-phosphor-image`, which returned `Accepted`.
Daemon PID 1195382 and instance `eceb92e7253969e488f7156c31d35d0f` identify this
attempt; verify the runtime record before any lifecycle action.
The [bounded current observation](../../artifacts/live-openbmc/romulus/image-build-resume-20260908.json)
records its jobs, typed events, retained warnings/errors and eventual terminal
result. This attempt failed util-linux linking: `more` had an undefined `main`
because `build/text-utils/more-more.o` was a zero-byte regular file dated 02:47.
The recipe source still defined main, and this was the only empty object in
that failed recipe's build directory. Yoctui retained both error log records
and the failed task. Cancellation through the exact instance/job was accepted
in 0.553 ms and produced a failed terminal result; this is an acknowledgement
observation, not a cleanup or release-performance measurement.

After the task was inactive, only that damaged object was moved to
`/home/bspguy-dev/.local/state/yoctui-openbmc-object-recovery.plNE35/more-more.o`.
It remains recoverable. The next image submission was accepted as job 2 in the
same daemon. It generated a 140 KiB object containing `T main`, linked `more`,
and completed util-linux compilation and subsequent package tasks. The
[fourth-attempt observation](../../artifacts/live-openbmc/romulus/image-build-object-recovery-20260908.json)
records this ongoing build separately. No image success is claimed.

The fourth attempt reached 2,854/6,812 before explicit recovery cancellation.
A read-only scan excluding sources and sysroots found 23 further zero-byte
compiled objects predating 06:00: function2 (8), boost (7), and fmt (8).
After confirming the exact private instance and running job 2, Yoctui accepted
cancellation in 235 ms and retained terminal failure. The workers stopped and
the idle daemon was stopped normally. Only those 23 objects were moved into
`/home/bspguy-dev/.local/state/yoctui-openbmc-object-recovery-23.bbZM2i`, preserving
their relative paths. They are recoverable; a subsequent check found no empty
objects in those three build trees. Source, sysroots and Poky were not changed.

A [real attached Tasks capture](../../artifacts/live-openbmc/romulus/resume-dashboard-20260908.txt)
also exposed an independent counter discrepancy. In a later job-2 snapshot at
sequence 1883, the job reported 2,339/6,812 but the retained build events held
only an older queued llvm_git statistic at 1,731 completed plus eight completed
rows. Task-event compaction drops newer aggregate statistics, so fresh client
replay undercounts completion. OPENBMC-SNAPSHOT-PROGRESS-001 owns the bounded
typed snapshot repair and regression tests, now verified in v0.1.76.

The tested candidate is preserved at
`/home/bspguy-dev/.local/state/yoctui-v76-validated.tWHDMp/yoctui`, SHA-256
`93515b28b1e5bd041ce9b486f376b0dde9a006b0d334d6ce5f2ff23851061d9f`.
Private daemon PID 1302830, instance `7f104be5e67613b8bb221198aea44aaa`, accepted
the fifth image attempt as job 1 after metadata readiness. Verify identity before
lifecycle actions. The normal installed release remains unchanged. The
[fifth-attempt observation](../../artifacts/live-openbmc/romulus/image-build-v76-recovery-20260908.json)
retains its own runtime evidence.

[Counter verification](../../artifacts/live-openbmc/romulus/v76-counter-verification-20260908.json)
brackets a [fresh production attach](../../artifacts/live-openbmc/romulus/v76-counter-attach-20260908.txt)
with snapshots 115 and 146. Both report aggregate and running-job counters of
2854/6812, exactly as the 160x50 UI renders, despite retaining only an older
1731/6812 task statistic. The client had no local build environment. Five focused
protocol/app/TestBackend regressions and all baseline checks pass; the full
workspace retry used two test threads after the existing PTY resize race failed
the first run. Tests were not weakened. OPENBMC-SNAPSHOT-PROGRESS-001 is DONE.
The image still runs; package/rootfs inspection and the complete release gate
remain outstanding under OPENBMC-LIVE-001. No release-performance claim is made.

The same restart exposed a separate job-identity defect: status after recovery
contained failed jobs 1 and 2, but the accepted fifth attempt became job 1,
replacing that historical record. The prior terminal evidence remains intact
in the third/fourth-attempt artifacts. Source inspection confirms independent
supervisor counters initialized at 1 and journal replacement by job ID alone;
this also permits cross-operation collisions. DAEMON-JOB-IDENTITY-001 tracks
checked shared allocation and isolated recovery/cancellation regression tests.
The current image will not be interrupted solely to deploy that correction.

The v0.1.78 candidate now requires a shared checked allocator at construction
for every non-Raw supervisor, seeded beyond retained IDs. The initial QA/
Security test reproduced two JobId(1) values. Six focused tests now pass:
cross-owner allocation, concurrent clones, namespace/exhaustion, persisted
history reduction, two isolated fake-bridge cancellation owners, and a real
private-daemon test that retains recovered jobs 1/2 while new QA/Security scans
receive IDs 3/4. The last test clears inherited environment/configuration and
uses only its private report fixture. All 1,556 workspace tests, 49 bridge tests,
strict Clippy, formatting, documentation, rasters and roadmap checks pass.
DAEMON-JOB-IDENTITY-001 is DONE. This is not a live image completion claim or
a deployment to the running daemon, which still uses the preserved v0.1.76.
The tested v0.1.78 binary is preserved at
`/home/bspguy-dev/.local/state/yoctui-v78-validated.RaKSXe/yoctui`, SHA-256
`185d14449e85f0397c80c53ce1ed5f63ec04c7282c57ab5aa08ffa6d5c9740fb`.

A later [reattachment capture](../../artifacts/live-openbmc/romulus/v78-reattach-timing-20260908.txt)
and [bounded findings](../../artifacts/live-openbmc/romulus/v78-reattach-findings-20260908.json)
confirm two separate defects. Snapshot 3784 still identifies LLVM do_compile
PID 1304077, whose process had run 2369 seconds before capture. The newly
attached v0.1.78 client shows 00:00:05 for both task and build and invents a
Started time of 04:49:13. Daemon lifecycle events lack timestamp fields and
app replay invokes local-clock reducers. OPENBMC-ATTACH-TIMING-001 owns the fix.

The same capture retains llvm_git as queued beside running llvm-native, plus
stale stdplus_git queue rows. The bridge's filename fallback strips only
underscore-digit versions; queue events supply taskfile while worker events
supply actual PN. Filename/version stripping cannot establish native/overridden
PN authority. OPENBMC-TASK-IDENTITY-001 owns bounded metadata-based identity
reconciliation and lifecycle regressions. Neither issue is an image-build
failure, and neither fix justifies interrupting the current image for deployment.

Record source revision, MACHINE/DISTRO, BitBake version, initialization command,
Yoctui version/hash, workspace paths, start/end timestamps and terminal outcome.
Start the image through Yoctui, not a separate unobserved BitBake invocation.
Inspect task transitions, live log rows, daemon/backend continuity, cancellation
and finished-duration behavior. After success, verify package inventory and
rootfs composition against the image's own generated metadata.

For each failure, distinguish upstream recipe/fetch/host errors from Yoctui
dispatch, capability, event or rendering defects. Add atomic registry tasks
with failing regression tests before fixing observed Yoctui defects, then
recheck against the same live environment. Keep fixture evidence separate from
real image-build evidence. Full completion remains pending until required
tasks and repository gates pass.
