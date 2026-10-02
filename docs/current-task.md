# Current Task

**ID:** XILINX-ZCU102-BUILD-001
**Title:** Build a real ZCU102 PetaLinux image through the Yoctui daemon
**Status:** IN_PROGRESS

Dependencies ENV and NATIVE-DISK-GUARD-OUTCOME-001 are DONE. Product v0.1.268
committed/pushed as ba76ae7aed30309a61f3c714cc6bcc4909cc8e8a; source-bound
optimized release SHA256 480b8cafc4e12d28c7585d5fed0b26714ec24089eca03166c472c94fca81f0ba.
Release debug0 saves host binary space only; target kernel DWARF stays enabled.
Isolated daemon 0b20c2dd7719990023902d8d24d1ada8 is ready. Real job2 guard
regression PASSED: native STOPTASKS/DiskFull, Failed/exit1, completed0/unknown
total, no manufactured failed task or successful100%. Genuine wide UI capture
shows Failed/Connected/full ZCU102 machine. Evidence native-disk-guard-fixed.txt.
Old job1 false-success history is retained, not image evidence.

User-approved removal of 1301 old top-level archive caches and matching markers
freed 4770631680 archive bytes (~4.4GiB); Git/source/image/debug files preserved.
This session's generated Cargo dev/release caches cleared with Cargo after all
checks; both releases retained/hash-verified. ~8.0GiB now free. Exact cleanup/
restore evidence is storage-cleanup.txt. GNOME desktop index remains temporarily
runtime-masked until storage permits restoration; private tag backup retained.

Fresh petalinux-image-minimal job3 is now RUNNING (accepted12:29UTC, native
start1790944300847ms). Actual UI checkpoint3005/10994 and two workers; IPC3008
at12:33:50UTC, no fetch failure. ~7.4GiB free. image-retry-v268.txt records
genuine UI/hashes; its6 displayed errors are retained guard diagnostics, not
observed current task failures. Do not submit another build or restart daemon.
Preserve/monitor the actual terminal outcome. Keep two workers,
rm_work exclusions and unchanged4GiB stop/1GiB halt guards. Do not duplicate or
restart an active build. Done requires actual successful task completion,
deployed ZCU102 rootfs/qemuboot/firmware/kernel and exact matching DWARF vmlinux/
config hashes. No image/vmlinux yet; QEMU/GDB remains NOT_STARTED and requires
BUILD DONE. No substitute machine, unmanaged build/QEMU, fake success or full
suite. If real storage/dependency blocks progress, record exact state and retain
all work, then select only eligible independent tasks. Product gaps remain
separate atomic versioned/focused-verified/pushed/source-bound release fixes.

Verification/monitoring:
```bash
./scripts/live-zcu102.sh build  # once only while idle
./scripts/live-zcu102.sh status
python3 scripts/inspect-zcu102-build.py | jq '{sequence,build_progress,jobs,last_build_events:[.last_build_events[]|del(.data)]}'
./scripts/live-zcu102.sh attach
./scripts/verify-roadmap.sh
```

## Historical blocked image validation handoff (superseded above)

XILINX-ZCU102-BUILD-001 is BLOCKED after the real disk guard stop. Historical
running checkpoints below are retained, not the latest terminal outcome. Native
source fetch/unpack and current debug config passed; no image/boot/GDB succeeded.

Dependency XILINX-ZCU102-ENV-001 is DONE. The retained official rel-v2026.1
environment is initialized in private Ubuntu 24.04 daemon instance
`d4a77149d273128063464c230720e618` (container PID 5073). Real recipe inventory
is ready; real release UI shows Connected/Local and zynqmp-zcu102-sdt-full.
ENV evidence/config/layer backups are in artifacts/live-xilinx/zcu102.

The actual image was accepted at 08:50 UTC on 2026-10-02 as daemon job 1.
After a separate cold build-backend parse it is RUNNING, not merely requested.
Real PTY captured 286/10994 tasks, two active workers and zero errors at 09:07;
read-only framed IPC reported 461 completed, no retained task failures and no
failed fetches at 09:13. These checkpoints do not certify a successful image.
Do not submit a duplicate build or restart the active daemon. Read current
aggregate authority with `python3 scripts/inspect-zcu102-build.py` (six focused
framing/identity/read-only checks pass); preserve the existing build to completion.
At 10:36 UTC it reached 1826/10994, still Running with zero fetch failures.
User-approved GNOME index pause/reset reclaimed ~9.5 GiB (~18.84 GiB now free).
All 216 tags/349 file associations are privately backed up; personal files are
untouched. `artifacts/live-xilinx/zcu102/storage-cleanup.txt` records cleanup,
private backup identity and REQUIRED runtime-mask removal/service restore after
validation. Desktop indexing remains temporarily paused, not permanently disabled.
Exact pinned Linux source is safely preseeded as a 272-MiB shallow Git cache:
`kernel-cache-preseed.txt` records source/tree identity, real fetch/lock install
and five focused Git guard checks. No active fetch/cache was overwritten, no
recipe/config revision or task stamp changed. This avoids an initial full-history
clone; actual daemon kernel fetch/unpack/build is still required. At 11:07 UTC
job 1 remained Running, 2411/10994, zero fetch failures, ~9.5 GiB free. Optional
older archive-cache cleanup (~4.4 GiB) awaits user approval; do not remove it yet.
Monitor real daemon tasks/outcome with `./scripts/live-zcu102.sh status` and
`./scripts/live-zcu102.sh attach`; preserve stdout/error/task logs. Keep two
task/make workers, four-GiB stop/one-GiB halt disk guards and rm_work exclusions
for linux-xlnx/petalinux-image-minimal. Native kernel fetch/unpack completed;
real source HEAD/tree exactly match the pin. Current resolved .config has
DEBUG_INFO=y/DWARF5=y and RANDOMIZE_BASE disabled (optional GDB_SCRIPTS disabled).
`kernel-configuring.txt` records exact config path/hash, real task stamps and a
passed 240-column live UI recapture; the 160-column shorthand-label predicate
failure is retained. Final config/ELF matching and image success remain pending.
Done requires actual successful image tasks,
deployed ZCU102 rootfs/qemuboot/firmware/kernel and exact matching ELF/DWARF
vmlinux/config hashes. Record pinned source IDs, real failure paths, storage
constraints and artifact identities. No mocked success, unmanaged QEMU, silent
alternative machine or physical-board action. On successful BUILD commit,
immediately continue XILINX-ZCU102-QEMU-GDB-001 through reviewed real UI flows.
Product gaps require separate atomic versioned fixes/focused checks/push and
source-bound release. Full suite remains deferred.

ENV verification passed: five capture-health checks, helper shell syntax and
failure guards, actual parse-only preflight (16539 files/0 errors), current
daemon workspace publication, live PTY/machine assertion, resolved kernel SCC/
fragment metadata, non-root user namespaces, reachable pinned hardware archive,
formatting/diff/roadmap checks. The vendor's optional qt-gui dangling append
warning is retained, not hidden. Two cold-inventory timeouts are retained too;
native parse-only preflight with exact daemon custom terminal variables warmed
the same cache without weakening startup bounds. Unused ROS layers are excluded
only from this build's BBLAYERS; coherent cloned sources and required layers
remain intact. Current release v0.1.267 checksum unchanged; no product code
change or full suite. Roughly 22 GiB free before build; guards must not weaken.

Historical ENV preparation/resume and earlier completed tasks follow:

**Resumed by the user after laptop shutdown — 2026-10-02.** The retained
container has restarted and isolated daemon instance
`130d73f2c2e8323f10bbfcbacc982af6` (container PID 41) is loading metadata.
Preparation is saved, not certified DONE: before shutdown initial daemon
compatibility/recipe discovery had not finished, and image build requests were
rejected with Conflict while it was loading. No image build, QEMU boot or live
GDB validation started. The isolated daemon was deliberately stopped and its
container stopped (not removed). Existing user daemon/environment untouched.

Resume from the retained checkout, not a new clone:

```bash
docker start yoctui-zcu102-validation-2026-1
./scripts/live-zcu102.sh start
./scripts/live-zcu102.sh status
```

Wait for actual compatibility/recipe authority, verify exact ZCU102 workspace,
then complete/commit ENV before proceeding to BUILD with
`./scripts/live-zcu102.sh build`. `./scripts/live-zcu102.sh attach` opens the
real UI in the supported Ubuntu 24.04 container. Keep two task/make workers,
disk guards, rm_work kernel retention and DWARF feature. Source/build roots,
exact commits, checksums, cleanup and captured UI are recorded in
`artifacts/live-xilinx/zcu102/environment.txt`; local.conf/bblayers.conf and
paused log/UI copies are committed alongside it. The complete checkout,
private runtime/state/logs and container-installed gdb-multiarch remain on disk.
Release v0.1.267 and its source-bound checksum are unchanged.

Known points to investigate, not completed fixes: explicit daemon --build-dir
discovery misses AMD's sources/poky layout (helper uses the supported inherited
setupsdk environment); AMD 2026.1 uses a firmware/multi-process QEMU launcher,
so inspect actual deployed artifacts before claiming the direct-kernel GDB
workflow supports it. No unmanaged QEMU or substitute qemuarm64 validation.
Capture health predicate was updated to recognize current Connected/Local
UI text while retaining legacy matching; focused checks pass. Verification:
`python3 scripts/test-live-capture-health.py`, `bash -n scripts/live-zcu102.sh`,
`git diff --check`, `./scripts/verify-roadmap.sh`. Full suite remains deferred.

New user validation supersedes the blocked queue. Dependency QEMU-GDB-UI-001 is
DONE. First measure/preview cleanup of known regenerable caches, preserving
user sources/artifacts, current release and existing daemon. Clone the newest
coherent official AMD PetaLinux manifest/layers; record commits and exact ZCU102
machine/provider. Prepare a new build with BB_NUMBER_THREADS=2 and make -j2,
bounded disk guards and rm_work that retains the matching debug kernel. Start
an independent Yoctui daemon with private runtime/state roots and verify its
initialized workspace/capabilities. Relevant new sources/build conf and live
evidence/scripts plus authoritative docs. Verify actual host/storage/license
prerequisites, config/task limits and daemon identity, then roadmap gate. Commit
coherent environment/evidence handoff and immediately begin the BUILD task.
No fake success, physical hardware mutation or full Rust test suite. Product
fixes, if needed, keep version bump/commit/push/source-bound release workflow.

Previous blockers/completed preparation are retained below; the old statement
of no eligible task describes the earlier handoff, not the new M111 request.

No eligible implementation task remains. An approved already-configured/halted
board, exact running-kernel .config/vmlinux and exclusive serial device/baud plus
target UART have not been supplied. No /dev/serial, ttyUSB or ttyACM target was
present on the read-only recheck. Do not flash/reset/configure, halt via SysRq,
or guess matching inputs to manufacture evidence. Manual verification: with
those exact approved inputs, review/attach and verify breakpoint/backtrace,
continue, separately approved manual re-entry, deliberate detach and reconnect.
Dependencies KGDB-SERIAL-PLAN-001/KGDB-SERIAL-UI-001 are DONE; mocks are not
physical-board evidence. Verification: manual approved-board workflow above
and ./scripts/verify-roadmap.sh; the user still defers the full suite.

KERNEL-INSTRUMENTATION-PLAN-001/UI-001 are DONE in v0.1.266/v0.1.267. Typed
closed presets and bounded reports, existing trapped form, correlated worker,
exact scrollable config/capability/fragment review, explicit second Enter
create-new export and edit/cancel/in-flight write boundaries are verified.
Only reported .config seeds input; output stays explicit. Missing is unknown;
matched config is not live compatibility. Failed export preserves inputs but
requires reinspection. No .config/layer edits, build/boot or detector self-tests.
Kernel model/app/UI/CLI 16/6/6/7, instrumentation pure/adapter 4/4, serial 3/4,
eight QEMU and four saved-environment checks pass. RootFS tree/systemd/chart
and Images scrolling regressions also pass. Strict affected Clippy, formatting,
UI spec, version policy, roadmap and optimized release pass. Final commit/push
and source-bound optimized rebuild deliver; installed PATH remains v0.1.250.
Use target/release/yoctui explicitly; daemon and user captures remain untouched.

KERNEL-INSTRUMENTATION-LIVE-001 is separately BLOCKED: provide an approved test
build/provider/version/architecture/compiler, supported fragment integration
and resolved .config, approved build/boot/reproduction and matching diagnostic
logs for each preset. Export/config match is not runtime evidence. M67's genuine
current-source Yocto performance capture remains independently blocked below.

Completed preparation verification (CARGO_INCREMENTAL=0; full suite deferred):
```bash
cargo test -p yoctui-model kernel_debug
cargo test -p yoctui-app kernel_debug
cargo test -p yoctui-ui kernel_debug
cargo test -p yoctui --bin yoctui kernel_debug
cargo test -p yoctui --bin yoctui kernel_instrumentation
cargo fmt --all --check
cargo clippy -p yoctui-model -p yoctui-app -p yoctui-ui -p yoctui --all-targets --all-features -- -D warnings
./scripts/verify-ui-spec.sh
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
cargo build --release -p yoctui --bin yoctui
```

Prior completed work and external blockers are retained below:

IMAGES-ARTIFACT-VIEW-001 is DONE in v0.1.265. Single-line responsive table
keeps all 80 selected long-name artifacts visible; exact byte sizes/readable
last-modified UTC metadata. o/e opens bounded internal text/DTS or the reviewed
DTB/DTBO decompile form; v opens existing authoritative IMAGE_ROOTFS Files.
Unsafe/binary/oversized files and missing dtc fail explicitly. Enter/p, build,
QEMU, associated files, rootfs chart/tree/systemd and terminal workflows remain.
Focused model/app/UI/CLI (17/1/2/6), 12 device-tree model, decompile UI/CLI,
five artifact adapter, existing Images/preview, RootFS and six GitUI checks pass.
Real local dtc roundtrip preserves its input and opens valid generated DTS.
Strict affected all-target/all-feature Clippy, fmt, UI spec, version policy,
roadmap and optimized build pass. A pre-existing allocation-only GitUI test
comparison was made borrowed for Clippy without changing the assertion.
Optional broader UI group remains 11 pass / two fail on both current source
and clean archived baseline f4960a14 (v0.1.264): editor Ctrl+S footer and SDK
legacy s/E shortcut assertions. They are unchanged, not weakened or hidden.
Final commit/push and commit-bound release rebuild deliver this change. No
full suite, disk-image mounts/extraction or daemon/build mutation occurred.
No eligible implementation task remains; KGDB board and M67 evidence remain
external blockers below. Installed PATH binary remains v0.1.250; launch the
workspace target/release/yoctui explicitly.

Completed M109 verification (CARGO_INCREMENTAL=0; full suite deferred):
```bash
cargo test -p yoctui-model image_artifact
cargo test -p yoctui-app images_workspace
cargo test -p yoctui-ui image_artifact
cargo test -p yoctui --bin yoctui image_artifact
cargo test -p yoctui-model device_tree
cargo test -p yoctui-ui dtc_decompile
cargo test -p yoctui --bin yoctui dtc_decompile
cargo fmt --all --check
cargo clippy -p yoctui --all-targets --all-features -- -D warnings
./scripts/verify-ui-spec.sh
python3 scripts/check-version-bump.py
./scripts/verify-roadmap.sh
cargo build --release -p yoctui --bin yoctui
```

Blocked queue and previous completed work retained below:

KGDB-SERIAL-PLAN-001 and KGDB-SERIAL-UI-001 are DONE in v0.1.263/v0.1.264.
Closed non-opening prerequisites, launch-time revalidation/exec, six-field form,
typed config/GDB preview and correlated worker integration pass focused checks.
Kernel model/app/UI/CLI (11/5/4/6), serial model/helper (3/4), QEMU, saved
environment, menuconfig, GitUI/modifier, RootFS and search regressions pass.
Formatting, strict Clippy, UI contract, version policy, roadmap and optimized
build pass; the actual release rejects unknown helper JSON without launching.
Final commit/push and commit-bound rebuild deliver this attach slice.

Live verification requires an approved already-configured/halted physical board,
exact running-kernel .config/vmlinux and exclusive host serial device/baud plus
target UART. No /dev/serial, ttyUSB or ttyACM target is present, and these inputs
have not been supplied. Reproduction/verification: use the new Kernel serial
form with those exact inputs, review and attach; verify breakpoint/backtrace,
continue, separately approved manual re-entry, deliberate detach and reconnect.
Fake-process/PTY checks are not physical-board evidence. Do not flash/reset,
trigger SysRq, change configuration or guess matching symbols to bypass this.
At this earlier M108 handoff no other implementation task was eligible. M110
now completes sanitizer/lockdep preparation only; live instrumentation remains
blocked and SysRq/kdump is still a proposal. Full suite remains deferred.

Previous user fixes (completed):

MODIFIER-SHORTCUTS-001 is DONE in v0.1.261 with focused app/model/UI/CLI
verification, strict Clippy, formatting, version policy and roadmap passing.
EDITOR-GITUI-CONTEXT-001 is DONE in v0.1.262 with exact-context bounded
asynchronous read-only inspection, correlated cancellation, retained dirty
editor state, contextual F12 routing and Ctrl+B then e restoration. Focused
model/app/UI/CLI checks, real GitUI 0.28.1 input/resize/exit smoke, workspace,
RootFS, search, QEMU and saved-environment regressions pass. Formatting, strict
Clippy, UI contract, version policy, roadmap and optimized build pass. Final
push and commit-bound optimized rebuild deliver both tasks. No full suite ran.

The separate M67 external blocker remains:

The retained performance evidence is not bound to the current source tree: it
has 143 previously documented source digest mismatches, including changes
predating M67. Supply a new genuine source/binary-bound Yocto 6.0.2 linux-yocto
compile capture using the documented release workload. Do not rewrite historical
digests or substitute fake-process startup timings for live evidence.

```bash
./scripts/verify-performance.sh --real-poky-evidence
./scripts/verify-completion.sh
```

This external prerequisite remains unresolved. The user
explicitly deferred the full suite; do not run the full completion suite without
a subsequent instruction. All M105/M106 requested corrections are DONE:
v0.1.258 target RootFS owner/group/mode, v0.1.259 systemd viewport scrolling,
and v0.1.260 single search activity marker. Focused verification and genuine
read-only Romulus ownership smoke pass. Seven optional wider UI assertions
fail identically on the previous commit and are documented, not weakened.
No overall completion/performance certification is claimed. Final push and
source-bound optimized binary handoff deliver these fixes; installed PATH
binary is still v0.1.250, so use target/release/yoctui explicitly.
