# Current Task

**ID:** XILINX-ZCU102-ENV-001
**Title:** Prepare an isolated current AMD PetaLinux ZCU102 validation environment
**Status:** IN_PROGRESS

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
