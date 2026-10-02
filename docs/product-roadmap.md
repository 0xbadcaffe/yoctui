# Yoctui Product Roadmap

## M114 — OpenBMC presentation/demo release polish

New user instruction selects profiling and measured hotspot fixes, correctness
polish without new features, full suite, README screenshots/flamegraph report,
operator guide, version bump, optimized installation, real OpenBMC screen/session
rehearsal and authenticated crates.io publication. Full-suite deferral revoked.
Atomic order: DEMO-PROFILE-001 -> DEMO-POLISH-001 (split implementation children
from actual findings) -> DEMO-FULL-VERIFY-001 -> DEMO-DOCS-001 ->
DEMO-INSTALL-LIVE-001 -> DEMO-PUBLISH-001. Preserve prior evidence, existing
workspaces and matching artifacts; no broad parent DONE from partial checks.
Profiling baseline DONE: fresh2299sample/6000frame stress plus real1179sample
idle OpenBMC profile. Local flag polling causes repeated full-App rollback
copies (37.32% inclusive); full suite88failures and real sibling-source platform
initialization error recorded. Atomic children now explicitly cover measured
background copying, selected-source initialization, current-contract fixtures,
verified UI clipping and isolated integration failures. None is silently DONE.
DEMO-LAYOUT-HYGIENE-001 separately reconciles the existing source-size/external
test-module violations before full release verification; no checker exemptions.
Selected-source init DONE v271 with real provider/firmware .config evidence;
DEMO-PLATFORM-CONFIG-001 fixes separately observed kernel config starvation
within existing bounded scans before contract fixtures; no quota increase.
That discovery fix is DONE v272 with exact unchanged105069byte live config
and focused scanner/fake bridge/UI tests; current task reconciles true contract
fixture drift while separating real Dashboard hint/footer/dialog defects.
The fixture parent now has separate model/app and UI-fixture children. UI
rendering polish follows model/app verification and precedes UI goldens; this
avoids a full-UI-golden dependency cycle without accepting real defects.
Focused rendering children precede the UI fixture child; the rendering parent
then verifies the full reconciled UI suite before integration work. The parent
retains its full-suite requirement rather than waiving stale golden failures.
Measured fixture stress is not real runtime/build-performance certification.
Unrelated physical/instrumentation/M67 gates stay explicit; ZCU102 deferred.

## M113 — OpenBMC/QEMU live kernel debugging and presentation evidence

User explicitly replaces current ZCU102 validation with retained OpenBMC
Romulus/QEMU. ZCU102 work is deferred/preserved, not deleted or marked DONE;
its two unfinished tasks are no longer required for this acceptance.
OPENBMC-QEMU-GDB-FLASH-001 is DONE in v0.1.269: reviewed closed flash boot mode using
existing managed session lifecycle, with private image staging and focused
default-direct-boot regressions. OPENBMC-QEMU-GDB-LIVE-001 is DONE after actual
U-Boot/Linux/OpenBMC boot, matching-symbol kernel breakpoint/backtrace/resume,
fresh-client reconnect/interrupt and owned cleanup on 2026-10-02. Exact input/
release identities, matching package-symbol recovery and retained native
warnings are in artifacts/live-openbmc/romulus/managed-flash-debug-v269.txt.
Four genuine screenshots/transcripts and editable presentation updates are
above the repo, not committed/pushed; slides17-19 show review/debug/boot.
No physical-board/sanitizer/destructive crash-test acceptance is inferred.
Full suite remains deferred; product fixes retain version/commit/push/release.

## M111 — Current AMD PetaLinux ZCU102 live acceptance

Resumed2026-10-02 by "finish kernel debugging"; pause revoked. Actual job3
is Failed/exit1, not draining: IPCsequence32861 retains3524/10994 and native
HALT909840384bytes. Daemon remains alive/idle. Only~5.6GiB currently available;
more safe storage is needed before retry. No new deletion/restart/build occurred.
Retained vmlinux.unstripped now has DWARF/symbols (hash/build ID in
resumed-kernel-debugging.txt), but final boot Image/deployed rootfs/qemuboot
are missing. Preserve partial kernel work; this is not image/debug acceptance.
Old compiler cleanup choice awaits user input; corrected3.2GiB build-directory
estimate includes binaries, while Rust4.1GiB is source and will be preserved.
AMD's pinned firmware/multiprocess helper passes unknown arguments to APU;
exact BOOT.bin/firmware/DTB, snapshot protection and owned cleanup still need
actual reviewed validation. Do not claim a new adapter gap fixed or live
compatibility from inspecting upstream code. Physical KGDB/instrumentation
live gates remain independently BLOCKED. No eligible independent registry work
or full-suite execution; current source-bound release remainsv0.1.268.

Historical paused checkpoint (superseded by resumed terminal outcome above):
Latest13:22UTC2026-10-02: user paused, resume hint ZCU102-RESUME. BUILD BLOCKED:
real job3 STOPTASKS at3.999GiB stopped new work; kernelcompile still draining,
jobRunning/exitnull, aggregate3524/10994 and~3.8GiB free. Actual terminal outcome
must be reread later. Approved cleanup exhausted; additional storage is required,
not weaker guards. No deployed image/matching vmlinux/QEMU/GDB success. Preserve
active private daemon/current kernel work; user pause overrides autonomous queue.

Historical: M112 native disk guard correction DONE in v0.1.268, pushed source-bound
release and real job2 Failed/exit1/unknown-progress regression verified with
native diagnostics/actual UI. Approved older archives (~4.4GiB) and own generated
Cargo cache cleanup leave ~8.0GiB; BUILD is IN_PROGRESS for a fresh isolated
daemon retry. No actual image, matching vmlinux, QEMU boot or GDB validation yet.
Evidence native-disk-guard-fixed.txt/storage-cleanup.txt. Guards unchanged.

Historical outcome before correction/approved cleanup: BUILD was BLOCKED by real storage guard, not DONE. At 11:29 UTC
job 1 soft-stopped on native STOPTASKS (~3.969 GiB), no deployed image/vmlinux,
~3.1 GiB free. Yoctui erroneously treated zero-failure completion as success and
forced total progress; current M112 NATIVE-DISK-GUARD-OUTCOME-001 fixes native
event authority independently. More storage/explicit retry is still required
for the actual image, then QEMU/GDB. Old archive-cache cleanup awaits approval.
Exact real failure/reproduction is in disk-guard-stop.txt. Earlier running
checkpoints below are historical; do not use final false 100% as success proof.

Native kernel fetch/unpack and current resolved config are now verified, not
only recipe metadata. Exact source HEAD/tree matches the pin; backend-created
fetch/unpack stamps and DEBUG_INFO/DWARF5 enabled / RANDOMIZE_BASE disabled are
recorded in kernel-configuring.txt. Optional GDB scripts are disabled. Passed
240-column actual UI recapture shows full ZCU102 identity/Connected/two workers/
zero errors; retained 160-column predicate failure is not claimed as passed.
Final compiled config/ELF, image/deploy and live QEMU/GDB remain pending.

BUILD job 1 is actually RUNNING through the private daemon (requested 08:50 UTC,
2026-10-02). Real release PTY recorded 286/10994 tasks, two workers and no errors;
subsequent framed IPC recorded 461 aggregate completions and no fetch failures.
Read-only inspector guards peer/workspace/instance and bounded partial frames;
six focused checks pass. Active-build evidence is retained, but deploy/matching
kernel and actual QEMU/GDB acceptance are still pending. Keep this build alive;
do not restart for the observed startup-versus-build terminal/cache mismatch.

ENV is DONE after resumed real verification: ready isolated daemon workspace,
exact ZCU102 machine displayed in real release PTY, coherent AMD source/config
identities and bounded workers/storage, validation-only fixed-address debug
fragment and vendor DWARF5 SCC in recipe metadata. Two cold inventory timeouts
and vendor qt-gui dangling append warning are retained in evidence. Optional ROS
is excluded only from this build's BBLAYERS; exact-daemon-environment parse-only
preflight completed 16539 files with zero errors and warmed the shared cache;
startup guards were unchanged. Helper/fmt/diff/roadmap checks pass, no full suite
or product code change. BUILD is current; real image/deploy/matching symbols and
reviewed QEMU/GDB validation remain required. Evidence: artifacts/live-xilinx/zcu102.

Historical shutdown resume checkpoint, superseded by ENV completion above:
User resumed execution after laptop shutdown on 2026-10-02. ENV remains
IN_PROGRESS; BUILD/QEMU-GDB remain NOT_STARTED. Coherent rel-v2026.1 clones,
bounded ZCU102 configuration and supported Ubuntu 24.04 validation container
are retained. Initial real daemon/UI connection exists, but discovery was
pending and build requests were rejected, not executed. Only the isolated
daemon/container were stopped. Resume commands and exact identities/evidence
are saved in current-task and artifacts/live-xilinx/zcu102; no full suite or
live image/boot/debug certification. The retained container/new isolated daemon
are running again; finish real metadata authority before requesting the build.

The user now explicitly authorizes a separate latest coherent meta-xilinx and
meta-petalinux checkout, initialization in Yoctui, real ZCU102 image build,
QEMU boot inside Yoctui and live kernel GDB validation. Safe measured cleanup
and reduced make/task parallelism precede the build. ENV, BUILD and QEMU-GDB
are independent atomic tasks; real output is required for each. This does not
complete physical-board KGDB or all four sanitizer/runtime evidence gates.
Release selection follows the current official [AMD manifest](https://github.com/Xilinx/yocto-manifests),
not mixed unrelated layer tips. Preserve existing daemon and user captures.

## M110 — Sanitizer and lockdep configuration preparation

The new continuation request activates step 3's preparation slice independently
of blocked physical-board verification. KERNEL-INSTRUMENTATION-PLAN-001 owns
closed presets/config report and bounded inspect/create-new export adapter;
KERNEL-INSTRUMENTATION-UI-001 owns existing Kernel form/review/worker integration.
Preparation tasks are DONE in v0.1.266/v0.1.267 with focused model/app/UI/CLI,
real filesystem-worker and existing QEMU/serial/saved-environment/RootFS/Images
regressions passing, plus strict Clippy/fmt/spec/version/roadmap/release checks.
Each has its own versioned commit, followed by push and a final commit-bound
optimized binary. KERNEL-INSTRUMENTATION-LIVE-001 stays BLOCKED:
provider/compiler/architecture-specific supported fragment integration, approved
build/boot/reproduction and matching runtime evidence have not been supplied.
No automatic layer/config edits, kernel build/deploy/reset or detector self-test.

Requested presets and limitations follow primary kernel documentation:
[KASAN](https://docs.kernel.org/dev-tools/kasan.html),
[KCSAN](https://docs.kernel.org/dev-tools/kcsan.html),
[UBSAN](https://docs.kernel.org/dev-tools/ubsan.html),
[lockdep](https://docs.kernel.org/locking/lockdep-design.html).
Vendor/version support must be confirmed by resolved Kconfig, not host tools.

## M109 — Usable deployed-artifact browsing

IMAGES-ARTIFACT-VIEW-001 is DONE in v0.1.265: selection-visible responsive
single-line table, exact bytes/readable UTC mtime, bounded internal o/e text/DTS
viewer, existing reviewed DTB/DTBO decompile form and v RootFS Files shortcut.
Model/app/UI/CLI focused checks (17/1/2/6), 12 device-tree model, decompile
UI/CLI, five artifact adapter, existing Images/preview, RootFS tree/systemd/
chart and six GitUI checks pass. Real local dtc roundtrip preserves its DTB.
Strict affected all-target/all-feature Clippy, fmt, UI spec, version policy,
roadmap and optimized build pass; final commit/push/source-bound rebuild deliver.
An inherited GitUI test allocation comparison was changed to a borrowed one,
not weakened. Optional broader UI group is 11 pass / two fail identically on
clean archived baseline f4960a14 (v0.1.264): editor Ctrl+S footer assertion and
SDK legacy s/E shortcut assertion. Tests are unchanged and no full suite pass
is claimed. Temporary baseline copy was removed; recoverable from Git. No
image mounts/extraction or daemon/build mutation. M108/M67 remain blocked.

## M108 — Physical-board KGDB serial attachment

KGDB-SERIAL-UI-001 is DONE in v0.1.264. Kernel model/app/UI/CLI checks
(11/5/4/6), serial planner/helper cases (3/4), QEMU model/backend/workspace
(2/8/3), saved environment seven, serial menuconfig fifteen, editor GitUI four,
modifier three, RootFS tree/systemd/chart and global search regressions pass.
Strict affected all-target Clippy, fmt, UI contract, version policy, roadmap
and optimized release pass; the actual binary rejects unknown-command helper
JSON before initialization. Native GDB default interrupt settings were inspected
without a connection (Ctrl-C sequence, interrupt-on-connect off). No board port
was opened, no target halt/resume, deployment/configuration write or live board
compatibility is claimed. Final push/commit-bound rebuild deliver the attach
slice. KGDB-SERIAL-LIVE-001 stays BLOCKED until an approved already configured/
halted board, serial path/baud/UART and exact config/symbols are supplied.
Read-only /dev discovery found no serial directory/ttyUSB/ttyACM device.

KGDB-SERIAL-PLAN-001 is DONE in v0.1.263. Three pure planner/config tests and
four CLI input/device/fake-process checks pass, with eight existing QEMU backend
regressions and strict affected all-target Clippy, fmt/version/roadmap passing.
Preflight reads no serial bytes and changes no inputs; launch-time checks reject
changed prerequisites before executing fixed GDB argv. The private helper child
test is invoked by the parent test, not substitute hardware evidence. Kernel UI
integration follows in KGDB-SERIAL-UI-001; board verification remains blocked.

The user requested continuing the Kernel debugging roadmap. The next authorized
slice is read-only prerequisite inspection and an explicitly reviewed serial
GDB client for an already configured/halted board. KGDB-SERIAL-PLAN-001 owns the
closed specification, bounded config/symbol/device checks and launch-time
revalidation helper; KGDB-SERIAL-UI-001 owns the existing Kernel form/preview
integration and versioned release. KGDB-SERIAL-LIVE-001 requires a real compatible
board, exact running-kernel config/vmlinux and approved serial transport. No
USB serial target is currently present; that verification is externally blocked.
Mocks/PTYs cannot certify board compatibility. Rebuild/deploy/reset, automatic
SysRq, serial break, privilege changes and configuration writes remain out of
scope. Sanitizers/lockdep were proposals at this M108 handoff; M110 now delivers
their configuration-preparation slice. Live instrumentation and SysRq/kdump
remain separate future/blocked work, not automatically completed by preparation.
Focused checks only, then independent versioned commits, push and commit-bound
release. The M67 performance blocker and user captures/daemon remain unchanged.

## M107 — Portable shortcuts and context-correct editor GitUI

User priority: MODIFIER-SHORTCUTS-001 adds typed Alt combinations and fixes
workspace-opening/search/GitUI routing/hints without consuming literal text or
native terminal keys. EDITOR-GITUI-CONTEXT-001 then enables repository launch
from every integrated source/layer editor using a bounded correlated read-only
probe and the existing destination chooser. Each gets focused regression tests,
documentation and an independent version-bumped commit. Push and final release
delivery follow both fixes. Full suite remains deferred and M67 blocked.

MODIFIER-SHORTCUTS-001 is DONE in v0.1.261. Modifier/Devtool/workspace input
checks (3/24/38), six keymap tests, 18 Devtool UI tests and native Alt/Unicode/
control/shift/release CLI checks pass, as do formatting, affected strict
all-target Clippy, version policy and roadmap. Text/PTY input and legacy
bindings remain unchanged; longer hints retain the action list and Git facts
in the responsive preview. Generic editor GitUI is completed separately below.

EDITOR-GITUI-CONTEXT-001 is DONE in v0.1.262. Exact-context bounded read-only
asynchronous inspection replaces the Devtool-only editor restriction. Missing
tool/nonrepository/invalid-path failures preserve edits; covered/replaced/stale
results cannot open a chooser. Embedded launch suspends the whole editor outside
the modal stack so native GitUI receives input. Ctrl+B then e restores the same
buffer/mode/cursor without stopping the terminal; detached/offline/cancel retain
the editor, and dirty retained state cannot silently be replaced.

Six new model cases and existing Devtool/search cases, four CLI worker cases,
menu/TestBackend checks across terminal sizes and actual GitUI 0.28.1 PTY smoke
pass. The native smoke exercises typed editor inspection/chooser/embedded launch,
Tab input, resize and clean exit, restores the dirty editor and verifies unsaved
disk contents are unchanged. Workspace/prefix/keymap, RootFS tree/chart/systemd,
global search, QEMU and saved-environment focused regressions pass. Formatting,
strict CLI dependency/all-target Clippy, UI contract, version policy, roadmap
and optimized release build pass. No full suite or live BitBake claim. Final
commit/push and commit-bound rebuild deliver both fixes; the daemon and user
builds/captures remain untouched. M67 remains separately blocked.

## M106 — Systemd list viewport and single search activity marker

Two independently committed user-reported fixes follow M105:
ROOTFS-SYSTEMD-SCROLL-001 keeps the selected service visible beyond the first
viewport; GLOBAL-SEARCH-MARKER-001 removes the duplicate static loading marker
beside the existing activity spinner. Preserve actions, focus and scope.
Each gets focused normal/boundary regressions and a version bump; release
delivery includes all queued fixes. Full-suite deferral and M67 blocker remain.
ROOTFS-SYSTEMD-SCROLL-001 is DONE in v0.1.259: selection-following bounded
rows, clipped title cue, all 80-service/page/reverse/wheel/resize and boundary
tests pass with unchanged typed actions. RootFS attributes and offline service/
bus regressions, strict affected Clippy and optimized build pass.
GLOBAL-SEARCH-MARKER-001 is DONE in v0.1.260: six global search, one workspace
search and twelve primitive tests pass, with RootFS regressions and genuine
Romulus ownership smoke passing again. Strict UI all-target Clippy, formatting,
version policy, roadmap and optimized build pass. Final commit/push and
source-bound release handoff include all three fixes; M67 remains separate.

### M106 focused verification and baseline limits

Systemd tests cover every selected row in an 80-service typed fixture, reverse
navigation, pages, Home/End, wheel mapping, resize, empty/short/stale and tiny
views. Search tests render global/workspace loading at multiple Unicode frames
with exactly one activity glyph, static reduced motion, ASCII, error/results,
cancel guidance and narrow safety. Default StateView markers remain unchanged.
No live-systemd management or search backend/scope changes are claimed.

The optional wider check `cargo test -p yoctui-ui palette_retains_typed_facts_at_every_breakpoint`
reports 17 pass / seven fail. A clean detached checkout of baseline 426f4e21
reproduces the same seven failures (16 pass / eight fail including its stale
global loading-scope assertion). The seven unrelated assertions remain unchanged:
`dialog_focus_is_trapped_then_visibly_restored_to_actionable_workspace`,
`next_generation_palette_bounds_scroll_and_clears_stale_empty_detail`,
`next_generation_palette_is_explicit_in_accessible_modes`,
`dashboard_renders_host_cpu_and_build_disk_space`,
`next_generation_palette_retains_typed_facts_at_every_breakpoint`,
`ux_command_center_unifies_bounded_source_contexts_without_bypassing_workspaces`,
and `compact_resource_meters_remain_visible_across_workspace_sizes`.
Baseline log: `/tmp/yoctui-palette-baseline-check.log`; its temporary checkout
was removed after verification. The relevant search assertion now requires the
already-existing `build and generated rootfs` scope and rejects the duplicate
static marker; scope itself was not changed. No full-suite pass is claimed.

## M105 — Correct target RootFS owner/group and mode

ROOTFS-TARGET-OWNERSHIP-001 is DONE in v0.1.258 and supersedes M100's host attribute presentation.
Use read-only exact-root Pseudo metadata and image account names, retain real
non-root identities, and expose missing/stale metadata without inventing root
or falling back to the build host. Keep navigation, previews, charts and Layers
unchanged. Focused checks only; the full suite remains deferred and M67 blocked.
Pseudo authority: [Yocto fakeroot/Pseudo](https://docs.yoctoproject.org/dev/overview-manual/concepts.html#fakeroot-and-pseudo).

### M105 real Romulus read-only ownership evidence

The adapter smoke on 2026-10-01 used the exact root
`/home/bspguy-dev/src/build-openbmc-romulus/tmp/work/romulus-openbmc-linux-gnueabi/obmc-phosphor-image/1.0/rootfs`
and sibling `pseudo/files.db`. Host lstat reports UID/GID 1000/1000. The adapter
reports `/etc/passwd` and `/etc/group` as `0644 root(0) root(0)`, and
`/etc/shadow` as `0400 root(0) root(0)` (host mode is 0600). No shadow contents
were read. Pseudo database bytes remained identical; daemon PID 1729515 stayed
alive. No BitBake job, mount, guest boot or daemon restart was required.

```bash
YOCTUI_ROOTFS_SMOKE_ROOT=/home/bspguy-dev/src/build-openbmc-romulus/tmp/work/romulus-openbmc-linux-gnueabi/obmc-phosphor-image/1.0/rootfs \
  CARGO_INCREMENTAL=0 cargo test -p yoctui-bitbake rootfs_browser_live_target_metadata_read_only_smoke -- --ignored --nocapture
```

Non-root 4242:73, target-only account names, numeric-only unknown names,
unavailable/stale/deleting/duplicate/corrupt/locked records, unsafe account
files and unchanged database bytes are independently covered by focused
fixtures. Live Romulus has no non-root Pseudo records; no live non-root claim
is made. This corrects generated-rootfs evidence, not a different deployed
artifact or running device. Custom/missing Pseudo locations remain unavailable.

## M104 — From Kernel guides to managed debug sessions

The user requested documentation of this roadmap and implementation of the
first QEMU → GDB step, now completed. The subsequent continuation request
authorizes M108's non-mutating physical-board serial attach slice; remaining
build/deploy/reset remain proposals; the latest continuation activates M110's
step-3 configuration preparation. It does not authorize a live target change.

1. **QEMU → GDB (current):** explicit deployed boot inputs and matching vmlinux,
   validated preparation, private debug transport, snapshot guest paused at
   startup, managed interactive GDB with bounded console logs, failure cleanup
   and real Linux guest breakpoint/backtrace/resume verification.
2. **Physical-board KGDB/KDB (M108 attach slice):** inspect kernel/transport prerequisites,
   review persistent Yocto configuration fragments and boot arguments, serial
   or approved proxy transport, explicit halt/resume and reconnect semantics.
   Board-specific build/deploy/reset actions need separate authorization.
3. **Sanitizers and lockdep (M110 preparation slice):** exact .config inspection,
   typed observations and reviewed standalone create-new .cfg presets. Manual
   provider-supported integration and resolved-config validation come next.
   Reviewed build/deployment, controlled reproduction, typed diagnostic capture
   and source navigation remain future slices. These are instrumented-kernel
   diagnostics, not interactive debugger sessions; each technique needs its own
   compatibility evidence. Preparation alone does not complete this broad step.
4. **SysRq/kdump (proposed):** separate read-only diagnostic collection from
   disruptive actions, inspect crash-kernel/memory/storage prerequisites, require
   explicit confirmation for halt/panic/reboot, then capture vmcore and hand off
   to the existing matching-symbol crash-analysis tool.

Sources: [QEMU GDB/Unix socket/security](https://www.qemu.org/docs/master/system/gdb.html),
[Yocto runqemu/snapshot](https://docs.yoctoproject.org/dev/dev-manual/qemu.html),
[KGDB/KDB](https://docs.kernel.org/process/debugging/kgdb.html),
[kernel debugging tools](https://docs.kernel.org/dev-tools/index.html),
[lockdep](https://docs.kernel.org/locking/lockdep-design.html),
[kdump](https://docs.kernel.org/admin-guide/kdump/kdump.html).

QEMU-GDB-SESSION-001 owns the typed plan/backend, QEMU-GDB-UI-001 owns Kernel
integration and versioned delivery, and QEMU-GDB-LIVE-001 owns genuine matching
Linux guest evidence. Tests do not substitute for unavailable real artifacts.
The existing M67 external evidence blocker and deferred full suite are retained.
All three QEMU → GDB tasks are DONE in v0.1.257. Final push and source-bound
optimized delivery follow; M67 performance certification remains separate.

### M104 live Linux verification — 2026-10-01

The genuine Poky 6.0.2 qemux86-64 guest used GDB 17.1 (Ubuntu 17.1-2ubuntu1)
and existing Yocto-native QEMU 10.2.0. No rebuild, daemon restart, deployment,
installation or configuration write was needed. Exact inputs below
`/home/bspguy-dev/src/build`:

| Input | Relative path | SHA-256 |
| --- | --- | --- |
| qemuboot | `tmp/deploy/images/qemux86-64/core-image-minimal-qemux86-64.rootfs-20260904162153.qemuboot.conf` | `c4329790944c2424a2ba1b9cd6622edc7a52876849c3ecbb35380c96adbfbaa2` |
| Boot kernel | `tmp/work/qemux86_64-poky-linux/linux-yocto/6.18.24+git/linux-qemux86_64-standard-build/arch/x86/boot/bzImage` | `642f72d65de1cf6f09fe72ff08e47a3a99355615398a6ce0123ac0376fc51a43` |
| Symbols | `tmp/work/qemux86_64-poky-linux/linux-yocto/6.18.24+git/linux-qemux86_64-standard-build/vmlinux` | `e178509f643ce040de0247df75d9b95bdae38d7d3b7bc5903b163cdd9a7995f6` |
| Rootfs | `tmp/deploy/images/qemux86-64/core-image-minimal-qemux86-64.rootfs-20260904162153.ext4.zst` | `ae8ab7187a802c11c9d4acb3b690a57ff7444897379803d2244163de7d8f02de` |

Runqemu: `/home/bspguy-dev/src/poky/scripts/runqemu`. Native QEMU relative path:
`tmp/work/x86_64-linux/qemu-helper-native/1.0/recipe-sysroot-native/usr/bin/qemu-system-x86_64`.
Memory 1024 MiB, TCG, four CPUs; vmlinux build ID
`826485cb9fd378a843ab7d1c57b0fadc1453e4a3`. The boot kernel comes from the
same build directory as vmlinux, not a newer unmatched deploy symlink.
Smoke debug binary SHA-256:
`adfefb5f0921d709aa141679b7c79ca25e3455076a305895d9ca512c437f9487`;
final optimized release is a separate source-bound delivery artifact.

Commands/results (backtrace excerpt):

```text
set pagination off
break start_kernel
Breakpoint 1 at 0xffffffff82e8ba60: file /usr/src/kernel/init/main.c, line 913.
continue
Thread 1 hit Breakpoint 1, start_kernel () at /usr/src/kernel/init/main.c:913
bt
#0 start_kernel
#1 x86_64_start_reservations
#2 x86_64_start_kernel
#3 secondary_startup_64
continue
[serial log] Run /sbin/init as init process
[serial log] Poky (Yocto Project Reference Distro) 6.0.2 qemux86-64 /dev/ttyS0
[serial log] qemux86-64 login:
Ctrl+C
Thread 4 received signal SIGINT, Interrupt.
bt
#0 pv_native_safe_halt
#1 arch_safe_halt
#2 default_idle
quit
y
Owned QEMU stopped. Console log retained.
```

Both managed helper and real Kernel form → preview → embedded daemon PTY routes
passed. Session 26 survived client exit at the kernel breakpoint; a fresh client
selected it, took writer control with Workspace focus, resumed to login,
interrupted and quit. Session became `Exited`; helper/runqemu/QEMU/GDB/guardian
PIDs were gone, socket and staged images were removed, and the 0700 directory
retained only its 0600 bounded log. Daemon PID 1729515 stayed alive throughout.

Retained local logs and SHA-256:

- Helper: `/tmp/yoctui-qgdb-7f6342a13105345fcb300056239f6d09/qemu.log` — `436b3c269d936d8cb9357f81187c447b921b529cf41011fc4d31d8c183ea3635`.
- Embedded/reconnect: `/tmp/yoctui-qgdb-9c23634fdd477e7504d4fa82a505b6aa/qemu.log` — `1cd199153ca1cbba0f0eadfd7886cb1753126621b1cc66b4b5d9b72adb1fa217`.
- Invalid boot header: `/tmp/yoctui-qgdb-26f79ee6c0456e089de6d5bb9122462f/qemu.log` — `41d3390fa0d4fd14a22b1f2a26f1a700edff765b00e582263050190fd2749331`.

The invalid-header smoke used a separate intentionally invalid `/tmp` image,
not the user's kernel. QEMU reported `invalid kernel header`; despite upstream
runqemu returning zero, the helper rejected early exit, stopped GDB and removed
owned socket/staging. Forced-helper-death cleanup passed focused fake-process
tests; no real-guest SIGKILL claim is made. Kernel/symbol/rootfs and Poky/Romulus
configuration hashes stayed unchanged. No smoke guest remains.

Limitations: DWARF points to unavailable `/usr/src/kernel`; source display needs
manual `set substitute-path`, while symbols/backtraces work. GDB's no-executable
warning is expected with symbols-only loading; native inferiors stay disabled.
No board, non-x86 or flash-only compatibility is claimed. The detached chooser
reuses the existing launcher; these boot smokes cover embedded/helper routes,
not a detached desktop window. They do not satisfy the M67 performance gate.

## M103 — Kernel debugging techniques and tools

KERNEL-DEBUG-TOOLS-001 and KERNEL-DEBUG-UI-001 are DONE in v0.1.256. Kernel has
a third Debugging tab with 12 typed tool routes and four non-executable guides,
explicit host/SSH scope, correlated preparation and exact embedded/detached
launch review. GDB disables init/auto-load, automatic debuginfod downloads and
implicit native target connection. Focused model/app/UI/CLI and regression
checks, strict affected-crate Clippy and harmless live GDB PTY smoke pass.
Actual target/kernel compatibility is not claimed by host-tool presence.
No full suite was run; M67 live performance evidence remains externally blocked.

## M102 — Persistent Hardware projects and bring-up

HARDWARE-PROJECT-STORE-001 and HARDWARE-PROJECT-UI-001 are DONE in v0.1.255:
real named project/subfolders, bounded non-overwriting import, persistent manual
stage progress, typed navigation/forms and restricted embedded previews. Native
Altium/Xpedition conversion is not implemented; graphical preview uses a
same-stem PDF export. Focused checks and live create/import/progress/restart smoke
pass. The full suite remains deferred; M67 live performance evidence stays blocked.

## M101 — Reload past build environments

SAVED-ENV-DAEMON-001 and SAVED-ENV-LOAD-001 are DONE in v0.1.254: independent,
diagnosable exact-profile daemon startup and reviewed History environment loading,
background start/attach/guarded idle replacement, fresh authority and adapter
rebinding. Focused verification and live Romulus start/attach/client-exit checks
pass; the full suite remains deferred. M67 live performance evidence stays blocked.

## M100 — RootFS file browser

ROOTFS-FILES-BROWSER-001 is DONE in v0.1.253: inline Layers-style lazy Files
tree, safe content preview and explicit host ls-style attributes. Focused
RootFS and Layers regression checks pass; full suite remains deferred per
user. M67 live evidence remains blocked.

REDUCE-UI-001 is split into four ordered tasks after an audit found 41,808
Rust lines, 17 production sources above 500 lines, five inline test modules and
18 oversized existing test sources. Workflow renderers are first, followed by
the core shell, remaining workspaces and final test migration.

REDUCE-APP-001 is DONE in v0.1.180. The final audit covers all 280 Rust files
under `yoctui-app/src`: every production and test file is at most 436 lines,
all 210 unit-test bodies live under `src/tests`, and public mapping/input paths
remain stable. Package tests, Clippy and repository gates pass. UI crate
decomposition is next.

REDUCE-APP-TESTS-001 is DONE in v0.1.179. Nine inline test modules and ten
oversized regression sources now live in 227 descriptive Rust files under
`yoctui-app/src/tests`. All 210 tests are preserved, no inline test module
remains, and every application Rust file is at most 436 lines. The final app
audit is next.

REDUCE-APP-DAEMON-JOBS-001 is DONE in v0.1.178. Daemon snapshot handling,
backend translation and build/Devtool coordination now flow through five named
sources; the largest is 315 lines. The production audit covers 54 application
sources with a 436-line maximum. All 210 tests and package Clippy pass. Test
migration is next.

REDUCE-APP-MAPPING-001 is DONE in v0.1.177. Raw protocol conversion, runner
events and compatibility snapshot mapping now flow through 10 named sources;
the largest is 313 lines. All 210 application tests and package Clippy pass.
Daemon and job coordination is next.

REDUCE-APP-INPUT-001 is DONE in v0.1.176. Mouse geometry, workspace keys,
dialogs and keyboard/menu routing now flow through 14 named sources; the largest
is 319 lines. All 210 application tests and package Clippy pass. Event mapping
decomposition is next.

REDUCE-APP-001 is split into four ordered tasks after an audit found 19,367
Rust lines, nine production sources above 500 lines, eight inline test modules
and ten oversized existing test sources. Input routing is first, followed by
event mapping, daemon/job coordination and final test migration.

REDUCE-BITBAKE-001 is DONE in v0.1.174. The final audit covers all 452 Rust
files under `yoctui-bitbake/src`: every production and test file is at most 500
lines, all 285 unit-test bodies live under `src/tests`, and public adapter paths
remain stable. Package tests, Clippy and repository gates pass. Application
crate decomposition is next.

REDUCE-BITBAKE-TESTS-001 is DONE in v0.1.173. All 45 inline test modules,
the source-root test module and the oversized regression source now live in 317
descriptive Rust files under `yoctui-bitbake/src/tests`. All 285 tests are
preserved, no inline test module remains, and every BitBake Rust file is at most
500 lines. The final BitBake audit is next.

REDUCE-BITBAKE-REMAINDER-001 is DONE in v0.1.172. Compatibility probing,
release fixtures, QEMU execution and SDK artifact discovery now route through
11 named sources. The production audit covers 136 BitBake Rust sources; every
production body is at most 497 lines. All 285 BitBake unit tests and package
Clippy pass. Inline test migration is next.

REDUCE-BITBAKE-BACKEND-001 is DONE in v0.1.171. Bridge transport, Devtool
execution and BitBake server lifecycle controllers now route through 11 named
process, event, graph, command, inspection and lifecycle sources; the largest
is 416 lines. All 285 BitBake unit tests and package Clippy pass. Remaining
oversized production sources are next.

REDUCE-BITBAKE-RUNNERS-001 is DONE in v0.1.170. SDK tools, layer QA,
self-test, security mapping and Raw jobs now route through 16 named capability,
command, validation, environment, output and runner sources; the largest is 463
lines. All 285 BitBake unit tests and package Clippy pass. Backend controllers
are next.

REDUCE-BITBAKE-REPORTS-001 is DONE in v0.1.169. Security, QA and test-result
report adapters now route through 15 named acquisition, parsing, validation,
command and runner sources; the largest is 478 lines. All 285 BitBake unit
tests and package Clippy pass. Workflow runner decomposition is next.

REDUCE-BITBAKE-ARTIFACTS-001 is DONE in v0.1.168. Wic, pkgdata,
rootfs composition and signature adapters now route through 17 named sources;
the largest is 458 lines. All 285 BitBake unit tests and package Clippy pass.
Report adapter decomposition is next.

REDUCE-BITBAKE-MAINTENANCE-001 is DONE in v0.1.167. Sstate, release,
service and optional maintenance adapters now route through 15 named capability,
validation, command, evidence, endpoint and runner sources; the largest is 466
lines. All 285 BitBake unit tests and package Clippy pass. Artifact and metadata
adapter decomposition is next.

REDUCE-BITBAKE-001 is split into seven ordered tasks after an audit found
44,502 Rust lines, 23 production sources above 500 lines, 45 inline test
modules, and an oversized existing test source. Maintenance adapters are first,
followed by artifacts, reports, runners, backend controllers, remaining sources
and final test migration.

REDUCE-PROTOCOL-001 is DONE in v0.1.165. The final audit covers all 113
Rust sources under `yoctui-protocol/src`: every production and test file is at
most 413 lines, all unit-test bodies live under `src/tests`, and public wire
paths and serde shapes remain stable. All 91 tests/doc-tests, Clippy and
repository gates pass. BitBake adapter decomposition is next.

REDUCE-PROTOCOL-TESTS-001 is DONE in v0.1.164. All eight inline test
modules moved into responsibility folders under `yoctui-protocol/src/tests`;
90 named test functions now have behavior-named files and the property test
remains with its shared fixture module. All 91 tests/doc-tests are preserved, no
test body remains in production sources, and every Rust source is at most 413
lines. The final protocol completion audit is next.

REDUCE-PROTOCOL-SUPPORT-001 is DONE in v0.1.163. Unix transport now
separates runtime/listener, connection, and path-security code; daemon
persistence separates recovery state from bounded private storage. The five new
sources are at most 308 lines, and every remaining protocol production source
is below 500 lines. Inline protocol test migration is next.

REDUCE-PROTOCOL-DAEMON-STATE-001 is DONE in v0.1.162. Client commands, QA
and testing messages, terminal messages, snapshot types, snapshot journal, event
reduction, and framing/errors now live in seven named sources; the largest is
413 lines. Wire ordering and all 91 protocol tests/doc-tests remain unchanged.
Transport and persistence support decomposition is next.

REDUCE-PROTOCOL-DAEMON-RAW-001 is DONE in v0.1.161. Raw request/history,
raw event/snapshot, compatibility identity and compatibility validation wire
families now live in four named sources of 249–373 lines. They remain in the
original daemon module, preserving public paths and serde representations. All
91 protocol tests/doc-tests and package Clippy pass. Daemon state is next.

REDUCE-PROTOCOL-001 is split into four ordered tasks after an audit found
8,831 Rust lines in seven sources, 3,205 production lines in `daemon.rs`, two
additional production sources above 500 lines, and eight inline test modules.
Daemon raw/compatibility types are first, followed by daemon state/framing,
transport/persistence support, and final test migration.

REDUCE-MODEL-001 is DONE in v0.1.159. The final audit covers 889 Rust
sources under `yoctui-model/src`: every production and test file is below 500
lines, every unit-test body lives under `src/tests`, and the largest source is
489 lines. All 492 model tests/doc-tests, Clippy, raw-catalog source mapping and
repository gates pass. Protocol decomposition is next.

REDUCE-MODEL-TESTS-001 is DONE in v0.1.158. All 31 remaining inline test
modules and the source-root test module now live in responsibility folders under
`yoctui-model/src/tests`. Eleven oversized grouped test sources were further
split into 208 behavior-named files. All model production and test sources are
now below 500 lines, with 486 unit tests and six integration tests preserved.
The final model completion audit is next.

REDUCE-MODEL-REMAINDER-001 is DONE in v0.1.157. The complete model
production audit found no source above 500 lines; the largest is 489 lines.
Public exports and reducer boundaries need no additional decomposition. Model
test placement is next.

REDUCE-MODEL-REDUCER-001 is DONE in v0.1.156. The main reducer is a
358-line exhaustive dispatcher and 13 former oversized transition owners route
to 38 bounded action-range modules. Guarded and fallback arms remain atomic,
and every reducer source is at most 367 lines. All model package tests/doc-tests
and package Clippy pass. The remaining oversized model-source audit is next.

REDUCE-MODEL-PROJECTIONS-001 is DONE in v0.1.155. Overview, dashboard,
progress, widget, daemon, PTY and session-update owners are thin coordinators
over 22 responsibility modules, all at 271 lines or less. Twenty-five tests
moved to meaningful files under `src/tests`, preserving state and projection
behavior. All model package tests/doc-tests and package Clippy pass. Oversized
reducer transition modules are next.

REDUCE-MODEL-INTERACTION-001 is DONE in v0.1.154. Text-area editing,
keymaps and operator action catalogs are thin coordinators over 17 responsibility
modules, all at 394 lines or less. Fifteen tests moved to meaningful files under
`src/tests`, with editing, binding and catalog behavior preserved. All model
package tests/doc-tests and package Clippy pass. Dashboard, progress and daemon
projection decomposition is next.

REDUCE-MODEL-WORKFLOWS-001 is DONE in v0.1.153. Root filesystem, Wic,
SDK, image artifact and project profile models are thin coordinators over 19
responsibility modules, all at 254 lines or less. Seventeen tests moved to
meaningful files under `src/tests`, with typed workflow and validation behavior
preserved. All model package tests/doc-tests and package Clippy pass.
Interaction model decomposition is next.

REDUCE-MODEL-APP-STATE-001 is DONE in v0.1.152. Application state and
actions are thin coordinators over 12 responsibility modules, all at 443 lines
or less. The 749 public action variants remain source-compatible, while state
construction, navigation, projections, workflows, menus and terminal accessors
have bounded owners. All model package tests/doc-tests and package Clippy pass.
Image and build workflow domain decomposition is next.

REDUCE-MODEL-COMPATIBILITY-001 is DONE in v0.1.151. Four compatibility
owners are thin coordinators over 24 responsibility modules, all at 489 lines
or less. Built-in capability definitions route through four closed families,
and 42 tests moved to descriptive files under `src/tests`. All model package
tests/doc-tests and package Clippy pass. Application state and action ownership
decomposition is next.

REDUCE-MODEL-SECURITY-TESTING-001 is DONE in v0.1.150. Security and testing are
small coordinators with 15 responsibility modules, all at 418 lines or less.
Security's 45 actions route through five exhaustive handlers, and 11 tests moved
under `src/tests`. All model package tests/doc-tests, package Clippy and full
repository gates pass. Compatibility model decomposition is next.

REDUCE-MODEL-QA-001 is DONE in v0.1.149. QA is a 29-line coordinator with 14
responsibility modules, all at 390 lines or less. Its 61 actions route through
seven exhaustive workflow handlers, and 16 tests moved to descriptive files
under `src/tests`. All QA-focused tests, package Clippy and full repository gates
pass. Security and testing model decomposition is next.

REDUCE-MODEL-MAINTENANCE-001 is DONE in v0.1.148. Maintenance is a 30-line
coordinator with 18 responsibility modules, all at 385 lines or less. The
56-action reducer is separated into nine typed workflow handlers, and 20 tests
now live in descriptive files under `src/tests`. Focused maintenance tests,
package Clippy and full repository gates pass. QA model decomposition is next.

REDUCE-MODEL-RAW-MODE-001 is DONE in v0.1.147. Raw mode is a 66-line
coordinator with 17 responsibility modules, all at 476 lines or less. Its 51
tests now live in descriptive files under `src/tests`; exact argv, authority,
preview, selector, favorites, history and execution behavior remains covered.
All raw-focused tests, catalog checks, package Clippy and full repository gates
pass. Maintenance model decomposition is next.

REDUCE-MODEL-RAW-CATALOG-001 is DONE in v0.1.146. The generated built-in raw
catalog is a 441-line coordinator and 50 deterministic command-family modules,
all at 450 lines or less. Generator checking covers the complete file set while
preserving 32 categories, 464 commands, 288 executable entries, ordering and
the reference hash. Focused raw tests, traceability/generator checks, package
Clippy and the full repository gates pass. Raw mode state decomposition is next.

REDUCE-MODEL-001 is split into 13 ordered responsibility tasks after an audit found 76,930 production lines, 31 files above approximately 500 lines and hundreds of inline tests. Built-in raw catalog decomposition is first, followed by raw state, maintenance, QA, security/testing, compatibility, application state, workflows, interaction, projections, reducers, remaining oversized sources and final test migration.

REDUCE-UTILS-001 is DONE in v0.1.144. Every utility production source is 105
lines or less, and 13 path, process, text, time and validation tests now live in
responsibility-named files under `src/tests`. The full package and serial
workspace suites, strict Clippy, fmt, 53 bridge tests, roadmap/version checks
and all 29 deterministic raster checks pass. The yoctui-model audit is next.

REDUCE-CLI-001 is DONE in v0.1.143. Every CLI production source is 504 lines or
less and all CLI test bodies live under `src/tests`. IPC and performance source
contracts now traverse the extracted daemon BitBake and client runtime modules;
all eight mutation/measurement checks and the direct log, task, IPC and
saturation contracts pass. The full package and serial workspace suites,
strict Clippy, fmt, 53 bridge tests, roadmap/version checks and all 29
deterministic raster checks pass. The yoctui-utils audit is next.

REDUCE-CLI-RUNTIME-TESTS-001 is DONE in v0.1.142. Twenty global-search,
tracing, PTY attachment, render/telemetry scheduler and PTY workflow tests now
live in responsibility-named CLI test files. The source-root PTY test file is
removed, and its shared fixtures remain in the test folder. The full package
and serial workspace suites, strict Clippy, fmt, 53 bridge tests,
roadmap/version checks and all 29 deterministic raster checks pass. The final
CLI audit is next.

REDUCE-CLI-DAEMON-WORKFLOW-TESTS-001 is DONE in v0.1.141. Seven QEMU, SDK,
security, test and WIC daemon workflow tests now live in responsibility-named
CLI test files with typed request identity and failure coverage preserved. The
full package and serial workspace suites, strict Clippy, fmt, 53 bridge tests,
roadmap/version checks and all 29 deterministic raster checks pass. Remaining
CLI runtime test migration is next.

REDUCE-CLI-DAEMON-ADAPTER-TESTS-001 is DONE in v0.1.140. Thirteen devtool,
maintenance, metadata and QA adapter tests now live in responsibility-named CLI
test files with their fake-process, cancellation and authority coverage
preserved. The full package and serial workspace suites, strict Clippy, fmt, 53
bridge tests, roadmap/version checks and all 29 deterministic raster checks
pass. Remaining daemon workflow test migration is next.

REDUCE-CLI-CORE-TESTS-001 is DONE in v0.1.139. Twelve archive, clone, job ID
and environment lifecycle tests now live in responsibility-named CLI test
folders with their fixtures and assertions preserved. The full package and
serial workspace suites, strict Clippy, fmt, 53 bridge tests, roadmap/version
checks and all 29 deterministic raster checks pass. Primary daemon adapter test
migration is next.

The final CLI audit confirms every source file is approximately 500 lines or
less; the largest is 504 lines. Four ordered child tasks now cover the remaining
inline-test migration across core lifecycle, daemon adapter, daemon workflow
and runtime helper modules before REDUCE-CLI-001 closes.

REDUCE-CLI-DAEMON-PTY-001 is DONE in v0.1.137. The 885-line daemon PTY owner now
has named supervisor routing, child runtime, request validation and typed
terminal mapping modules, all below 500 lines. Seven unit tests and the ignored
real-GitUI smoke test moved to descriptive CLI test files while preserving
platform gates and PTY lifecycle behavior. The serial 1,692-test workspace suite
(five existing ignored), strict Clippy, fmt, 53 bridge tests, roadmap/version
checks and all 29 deterministic raster checks pass. The final CLI source audit
is next.

REDUCE-CLI-DAEMON-RAW-001 is DONE in v0.1.136. The 1,465-line raw execution
owner now has named job, PTY, attachment/cancellation control, event reduction
and recovery modules, all below 500 lines. Six inline tests moved to descriptive
CLI test files while preserving exact request/session identity, lifecycle and
owned child behavior. The serial 1,692-test workspace suite (five existing
ignored), strict Clippy, fmt, 53 bridge tests, roadmap/version checks and all 29
deterministic raster checks pass. Daemon PTY ownership is next.

REDUCE-CLI-DAEMON-ROOTFS-001 is DONE in v0.1.135. The 1,011-line daemon rootfs
owner now has named client IPC, authority validation, worker ownership and
source acquisition modules, all below 500 lines. Six inline tests moved to
descriptive CLI test files while preserving full query identity, stale retries,
cancellation and owned-process reaping. The serial 1,692-test workspace suite
(five existing ignored), strict Clippy, fmt, 53 bridge tests, roadmap/version
checks and all 29 deterministic raster checks pass. Raw daemon execution is
next.

REDUCE-CLI-DAEMON-COMPAT-001 is DONE in v0.1.134. The 1,387-line daemon
compatibility owner now has named coordination, runtime detection and bounded
process helper modules, all below 500 lines. Eleven inline tests moved to
descriptive CLI test files while preserving fail-closed authority, stale-probe
and process-bound checks. The serial 1,692-test workspace suite (five existing
ignored), strict Clippy, fmt, 53 bridge tests, roadmap/version checks and all 29
deterministic raster checks pass. Daemon rootfs inspection is next.

REDUCE-CLI-DAEMON-BITBAKE-001 is DONE in v0.1.133. The 1,419-line daemon
BitBake supervisor now has named lifecycle, bounded ingress, activity
notification and cancellation modules, all below 500 lines. Ten inline tests
moved to descriptive CLI test files while preserving embedded Python fixtures.
The serial 1,692-test workspace suite (five existing ignored), strict Clippy,
fmt, 53 bridge tests, roadmap/version checks and all 29 deterministic raster
checks pass. Daemon compatibility inspection is next.

REDUCE-CLI-CLIENT-TRANSPORT-001 is DONE in v0.1.132. The 798-line daemon client
transport now has named handshake, attachment lifecycle and messaging/polling
modules, all well below 500 lines, with four integration-style tests moved to
descriptive files under the CLI test folder. The serial 1,692-test workspace
suite (five existing ignored), strict Clippy, fmt, 53 bridge tests,
roadmap/version checks and all 29 deterministic raster checks pass. Daemon
BitBake execution is next.

REDUCE-CLI-CLIENT-RUNTIME-001 is DONE in v0.1.131. The 1,485-line daemon client
runtime now uses named attach, replica polling, typed effect input/routing and
terminal control modules, all below 500 lines. Thirteen inline unit tests moved
to descriptive files under the CLI test folder. The serial 1,692-test workspace
suite (five existing ignored), strict Clippy, fmt, 53 bridge tests,
roadmap/version checks and all 29 deterministic raster checks pass. Client
transport is next.

REDUCE-CLI-MAINTENANCE-001 is DONE in v0.1.130. The former 2,713-line
maintenance source is split into shared state/inspection, coordinator polling,
typed preview and operation modules, all below 500 lines. Its 17 inline workflow
tests now live as named files under the CLI test folder. The serial 1,692-test
workspace suite (five existing ignored), strict Clippy, fmt, 53 bridge tests,
roadmap/version checks and all 29 deterministic raster checks pass. Client
runtime is next.

The remaining REDUCE-CLI-001 scope is split into eight ordered implementation
tasks: maintenance, client runtime, client transport, daemon BitBake,
compatibility, rootfs, raw execution and PTY ownership. A final CLI audit
completes the parent task after those file families and their inline tests move.

REDUCE-CLI-TUI-001 is DONE in v0.1.128. The 3,755-line interactive runtime
is now a 377-line startup module with one typed state owner and named polling,
event, input-route, job and shutdown modules. Every runtime source remains near
500 lines or below; ordered key stages retain explicit handled and outer-loop
continuation outcomes. Performance source contracts now cover the extracted
runtime tree. The 1,692-test workspace suite (five existing ignored), strict
Clippy, fmt, terminal harness, 53 bridge tests, roadmap/version checks and all
29 deterministic raster checks pass. Remaining yoctui-cli source families are
next.

REDUCE-CLI-DAEMON-001 is DONE in v0.1.127. Daemon orchestration is 451
lines; private background, telemetry, client-message and twelve command-family
modules each remain below 500 lines. Typed daemon/client ownership retains
request order and explicit no-immediate-reply handling. All 43 command arms
retain their original tokens after accounting for state qualification, deferred
returns and rustfmt wrapping. A real private-daemon test checks one reply and
connection continuity across eleven rejected command families, including the
build-directory authority rejection. The serial workspace suite (1,692 tests,
five existing ignored), strict Clippy, fmt, 53 bridge tests, eight source-checker
tests, version/roadmap and 29 deterministic raster checks pass. Historical live
evidence is preserved. Interactive runtime decomposition is next.


REDUCE-CLI-MAIN-001 is DONE in source v0.1.126: main.rs is 468 lines,
with 72 responsibility-named private modules. Its inline tests and fixtures
now live under src/tests; all 491 extracted function bodies match the original
parsed tokens before the necessary moved include_str fixture-path correction.
The full serial workspace suite passes (1,691 tests, five existing ignored),
strict workspace Clippy, fmt, 53 bridge tests, eight IPC/measurement checker
tests, docs/version/layout/roadmap checks and all 29 deterministic rasters pass.
The existing CLI description is preserved and its stale documentation assertion
is aligned. Historical evidence and the four user captures remain untouched.
REDUCE-CLI-DAEMON-001 is next, followed by the interactive loop and crate review.


## M73 — Source decomposition

Review every source file, starting with main.rs, target approximately 500 lines
per file, use responsibility-based names, and move inline tests to test folders.
Preserve public interfaces, runtime behavior and every existing assertion.

## M72 — Live dashboard corrections

Completed the reported OpenBMC client continuity, compact resource/cache
status, readable progress and responsive Insights tabs. M72-IPC-001,
M72-CACHE-001 and M72-LAYOUT-001 are DONE with baseline and reviewed screenshot
verification. The running user build is preserved; protocol 1.3 requires a
matching rebuilt daemon and client after the active build finishes.

This roadmap defines the stable milestone sequence. Atomic implementation state lives in `docs/task-registry.toml`.

## Current handoff

M72 is complete: 758 tasks are DONE; M67-LIVE-EVIDENCE-001 remains BLOCKED
on genuine current-source real-Poky performance evidence. No release-performance
certification is inferred from fixture tests or rewritten historical hashes.

README-DBUS-UDEV-001 is complete in source v0.1.122. The README has 23 verified
screenshots, including system D-Bus activation files and udev rules/overrides.
753 tasks are DONE; M67-LIVE-EVIDENCE-001 remains BLOCKED.

README-SYSTEMD-001 is complete in source v0.1.121. The README has 21 verified
screenshots, including offline systemd service files beside rootfs composition.
752 tasks are DONE; M67-LIVE-EVIDENCE-001 remains BLOCKED.

README-REAL-DTS-001 is complete in source v0.1.120. The Device Tree screenshot
uses attributed upstream Linux v6.6 DTS content and tested syntax highlighting.
751 tasks are DONE; M67-LIVE-EVIDENCE-001 remains BLOCKED.

README-ONBOARDING-001 is complete in source v0.1.119. The README follows the
new-user setup and daily development workflow, preserves all 20 screens and
technical information, and removes release-update and promotional wording.
750 tasks are DONE; M67-LIVE-EVIDENCE-001 remains BLOCKED.

M67/M68/M69 requested implementation and UI repairs are complete. Version
0.1.118 is published across all seven crates on crates.io and includes offline
navigation, durable saved build history/log excerpts and the reviewed 20-screen
README gallery. The remaining BLOCKED task is:
M67-LIVE-EVIDENCE-001 requires a genuine current-source real-Poky performance
capture before performance certification.

## Product completion rule

README-VERSION-BADGE-001 follows the user's correction: the README title is
plain Yoctui and the release version lives only in the registry-backed badge.
v0.1.97 passed hosted run 34258366108 and is published across all six crates;
v0.1.98 is the documentation-only follow-up, not a new registry release.
README, docs, version, fmt and deterministic raster checks pass; all 715 tasks
are DONE.

RELEASE-CI-097 pins the Python quality tools to the validated environment after
the actual hosted tool-version drift. All 714 tasks are DONE; v0.1.97 is the
candidate and hosted baseline/package verification gate publication.

RELEASE-CI-096 supplies the test container's init reaper after a reproduced
process-lifecycle failure. All 713 tasks are DONE; v0.1.96 is the candidate
and hosted baseline/package verification remains required before publication.

RELEASE-CI-095 explicitly provisions rustfmt and Clippy in the minimal test
container after the actual v0.1.94 hosted result. All 712 tasks are DONE;
v0.1.95 is verified locally and awaits hosted CI and final package validation.

RELEASE-CI-094 completes the follow-up to hosted v0.1.93 results: compatibility
and PTY repairs pass; container checkout trust and snapshot imports are now
corrected and verified locally. All 711 tasks are DONE. v0.1.94 is the current
publication candidate, pending hosted CI and final locked package verification.

RELEASE-CI-093 completes the user-requested v0.1.93 README/version update,
hosted CI repair and publication-package verification. All 710 tasks are DONE;
the candidate is ready for push, hosted CI observation and registry publication.
The earlier completion run was stopped during its optimized UI benchmark;
the final completion gate remains pending.

All 709 registry tasks are DONE. RELEASE-IPC-GATE-001 is complete after its
two atomic follow-ups: v0.1.90 RELEASE-IPC-SOURCE-001 repaired the obsolete
fail-closed constructor assertion, and v0.1.92 RELEASE-FIXTURE-READY-001 added
bounded read-only metadata readiness with fresh source-bound flood and
30-minute memory evidence. No runtime source, threshold, workload or timer was
changed by either checker/harness repair. The repository-wide completion gate
must still pass in a clean worktree as the final release confirmation.

v0.1.87 closes real OpenBMC integration with actual image/artifact/lifecycle/
inspection evidence. v0.1.89 completes RELEASE-DAEMON-CPU-001 and
RELEASE-PERF-REFRESH-001 with a bounded telemetry publication repair and actual
fresh source-bound idle/real-Poky acceptance. All 705 registry tasks are DONE.
Combined real-build CPU is 0.5831% of one logical CPU over 360 samples; input
p95 is 4.558 ms, and continuity/cancellation pass. The full real-Poky performance
verification chain and baseline pass. All release thresholds remain in force;
the complete repository gate is next and has not yet passed.

v0.1.88 fresh real-Poky evidence fails combined CPU at 1.0334662486% versus
1.00% over 360 samples, while latency, saturation, continuity and cancellation
pass. RELEASE-DAEMON-CPU-001 was registered before profiling and runtime edits.
That failed evidence remains preserved alongside the separate diagnostic
profile and actual passing v89 observation; thresholds are unchanged.

v0.1.86 completes OPENBMC-ROOTFS-SOURCES-001: bounded asynchronous exact-image
queries honor the daemon's selected command/API authority. Actual rootfs and
services screens now work, with independently matched 2628 entries/81096497
bytes and available package/offline system metadata. The only remaining partial
limitation is unknown filesystem package ownership. All 1595 workspace tests/
doc-tests and baseline checks pass. OPENBMC-LIVE-001 resumes for integration
handoff and the completion gate, including fresh source-bound real-Poky
performance evidence. No rebuild or firmware boot is claimed.

v0.1.85 resolves all 40 renamed-package metadata gaps: the production image
screen now reports 228 installed packages available, 81062873 bytes and 1778
files. OPENBMC-PKGDATA-001 is DONE with four new regressions, all 1584 workspace
tests/doc-tests, 281 UI tests, 52 bridge tests and full baseline verification.
OPENBMC-ROOTFS-SOURCES-001 is current; its missing attached IMAGE_ROOTFS lookup
still prevents complete image UI validation. No image rebuild was needed.

v0.1.84 records successful real Romulus image completion: 6812/6812, job 1
exit 0, generated flash/SquashFS artifacts, 228 packages and retained rootfs.
New-daemon native/git lifecycle, recovered job IDs and frozen reattachment
timing pass actual BitBake checks. Image UI inspection exposed two follow-ups:
OPENBMC-PKGDATA-001 resolves runtime-reverse package names;
OPENBMC-ROOTFS-SOURCES-001 supplies exact recipe metadata to attached clients.
OPENBMC-LIVE-001 waits for both and their real UI rechecks. The single upstream
render-group warning was investigated, not suppressed; no firmware boot or
completion-gate success is claimed.

The following integration chronology describes the prerequisites at each
version; its earlier pending image claims are superseded by the result above.

M56 real OpenBMC validation additionally requires bounded large-inventory
transfer (OPENBMC-INVENTORY-001): live Romulus parsing exposed a recipe response
above the bridge's 1 MiB frame limit. v0.1.69 now transfers the complete real
inventory in bounded chunks; v0.1.70 fixes bounded CLI stale retries and rejection
status. v0.1.71 fixes symlinked PATH tool discovery. The image build is next; inventory and
submission tests are not image-build validation.

The first real OpenBMC attempt exposed a host pyenv-shim recursion loop. The
user-requested persistent host repair is complete (HOST-PYTHON-001), preserving
managed interpreters while defaulting build shells to system Python. A separate
initial-attachment timeout repair (OPENBMC-ATTACH-001) is complete in v0.1.73;
the real image retry and subsequent inspection remain required.

Live validation also exposed snapshot compaction and reattachment defects.
Aggregate counters (v0.1.76), shared recovered job identities (v0.1.78), and
observed lifecycle timing (v0.1.80) now pass their regressions and baseline
checks. Queue/worker recipe identity (v0.1.81) now passes bounded metadata,
unknown-identity and cross-layer regressions plus the baseline. The real image still
runs on the preserved v0.1.76 daemon. These fixes do not claim image completion
or waive the source-bound performance and release gates. A subsequent real
attachment exposed Workers: 0 beside two active tasks. OPENBMC-WORKER-COUNT-001
is DONE in v0.1.83 with typed active-worker authority, eight focused regressions,
the full baseline and real attached display matching two known active PIDs.
At v0.1.83 OPENBMC-LIVE-001 resumed; v0.1.84 records its successful image and
the two remaining image-inspection repairs above.

Yoctui is 100% complete only when:

- every required task in `docs/task-registry.toml` is `DONE`
- `./scripts/verify-completion.sh` passes
- the supported live Yocto/BitBake compatibility matrix has been validated
- no required workflow is represented only by a placeholder
- documentation matches the shipped behavior

## M0 — Governance and reliable execution

Goal: a fresh agent can continue implementation without inventing scope or losing progress.

Exit criteria:

- root `AGENTS.md`
- one active task in `docs/current-task.md`
- machine-readable task registry
- human-readable implementation status
- roadmap verification
- objective final completion gate
- architecture and UI specifications treated as contracts

## M1 — Reliable BitBake cockpit

Goal: reliably control and observe real builds.

Capabilities:

- workspace discovery
- bridge and process backends
- build start and cancellation
- parse and task lifecycle
- bounded logs
- structured errors
- build history
- CPU, memory, and disk telemetry
- responsive CPU/memory/disk gauges, bounded history sparklines, load averages,
  and honest average task velocity/ETA
- terminal restoration
- validated live BitBake compatibility

Exit criteria:

- real build smoke tests on supported versions
- normal completion, failure, cancellation, and bridge loss tested
- typed backend-to-model event contract enforced
- fractional process progress and PID-only task progress normalize without
  disconnecting the typed stream, and determinate task progress renders as bars
- daemon-owned builds restore and continuously update the same typed build and
  task cockpit after clients detach and reattach

## M2 — Persistent Yocto workbench

Goal: navigation remains useful while jobs run.

Capabilities:

- persistent shell
- responsive wide, medium, narrow, and too-small layouts
- shared focus router
- dialog stack
- command palette
- contextual footer
- themes and accessibility preferences
- notifications
- persistent background-job model
- Tasks, Logs, Errors, Settings, and Images workspaces

Exit criteria:

- all long operations survive workspace navigation
- all dialogs trap focus
- no terminal size causes a panic

## M10 — Optional project profiles

Goal: let teams record portable Yoctui intent in an optional
`.yoctui/project.toml` without changing vendor/layer metadata or replacing
BitBake authority. Profiles contain typed favorites, presets, and workflows;
they never execute arbitrary shell text when loaded, and personal settings
remain user-local.

The profile contract is versioned and fail-closed. References are portable and
repository-relative, stale authoritative identities remain explicit, loading
is inert, and generation is an explicit reviewed action. Real-Poky acceptance
must cover both an unmodified no-profile checkout and an explicitly generated
profile before compatibility is claimed.

## M11 — Persistent daemon and session architecture

Goal: provide one Rust-native, terminal-native Yoctui daemon that owns BitBake,
background work, and PTYs while attachable Ratatui clients render and control
that state. Local IPC is the default; SSH clients attach on the build host
without opening an unauthenticated TCP service. Client disconnect, SSH loss,
daemon restart, and host reboot semantics must be explicit and honest.

## M3 — Recipe, layer, metadata, and dependency development

Goal: complete daily recipe and layer work without leaving Yoctui except for intentional editor/terminal launches.

Capabilities:

- lazy layer tree
- file preview and editing
- Git decorations and refresh
- recipe search and actions
- configuration provenance
- Devtool lifecycle
- task and recipe dependency exploration
- signature inspection
- package-data browser
- recipetool workflows
- bitbake-layers diagnostics

## M4 — Images, packages, SDK, QEMU, and Wic

Goal: build, inspect, run, and deploy images and SDKs.

Capabilities:

- image artifacts
- package membership
- SDK generation and publication
- managed QEMU sessions
- Wic creation
- protected device writing
- native tool and extracted-SDK workflows

## M5 — Testing, QA, CVE, and SPDX

Goal: make validation and security workflows first-class.

Capabilities:

- oe-selftest
- bitbake-selftest
- testimage
- testsdk
- ptest
- resulttool
- typed result regression comparison and JUnit export
- CVE analysis
- SPDX/SBOM
- kernel config checks
- URI, patch, and license QA
- layer QA

## M6 — Maintenance and release engineering

Goal: safely expose advanced maintenance.

Capabilities:

- sstate readiness and cleanup
- PR service diagnostics and tools
- hash server diagnostics
- locked signature generation
- build comparison
- Git archive
- optional pull-request workflows
- repo manifest integration
- Toaster detection

## M7 — Production hardening

Goal: release-quality reliability.

Exit criteria:

- formatting, lint, tests, coverage, audit, and deny pass
- property, fuzz, stress, terminal, and process-tree tests
- deterministic profiling and memory reports
- a fresh representative release flamegraph with resolved application stacks
- complete compatibility matrix
- installation and operator documentation
- final completion gate passes from a fresh checkout

## M8 — In-app build environment onboarding

Goal: make first-run BitBake setup safe and understandable without requiring
shell setup before starting Yoctui.

Capabilities:

- launch without a build-directory argument
- typed existing-source and reviewed Poky-clone profiles
- validated environment initialization and interactive setup-shell handoff
- managed child-only environment capture
- explicit BitBake connection verification before build controls unlock

Exit criteria:

- startup never treats the current directory as an implicit build
- clone, initialize, cancel, shell, and verification outcomes remain distinct
- no build or metadata action starts before a typed connection succeeds

## M12 — crates.io distribution

Goal: ship the first installable public release as `yoctui` 0.1.0.

Capabilities:

- self-contained bridge behavior after `cargo install yoctui`
- complete crates.io metadata and bounded package contents
- publishable internal dependency crates with private test/support crates excluded
- dependency-ordered, reproducible publication and clean-install smoke validation

Exit criteria:

- the packaged binary does not depend on repository-relative runtime files
- every public package passes packaging and isolated build checks
- `yoctui` 0.1.0 is published under the intended crates.io account
- a clean registry installation runs the published binary successfully

## M13 — Dense terminal workbench redesign

Goal: make the persistent Ratatui client match the approved compact IDE-style
Yocto operations workbench while preserving typed behavior and accessibility.

Capabilities:

- one-line project and daemon/BitBake status header
- grouped IDE-style Navigator with full-row selection
- compact panel chrome and contextual command rail
- three-tier Tasks cockpit with live log and retained job history
- structured task Inspector with context actions and system status
- wide, medium, narrow, no-color, and reduced-height regression coverage

Exit criteria:

- the default dark workbench preserves the approved blue/lime/amber visual hierarchy
- all displayed values come from typed model or daemon replica state
- keyboard, mouse, focus, theme, and responsive contracts remain intact
- deterministic TestBackend and PTY snapshot checks pass

## M14 — Live workspace usability recovery

Goal: make the first real Poky launch reliably show the approved workbench and
metadata without hidden state from tests or prior diagnostic invocations.

Capabilities:

- launch-scoped backend and no-color overrides
- isolated PTY/snapshot configuration and runtime state
- metadata-capable local bridge fallback when the daemon is absent
- non-obscuring daemon-disconnected status
- directly discoverable theme selection and explicit pane-focus routing
- live Poky metadata and PTY visual acceptance

Exit criteria:

- a normal launch against `~/src/poky/build` shows workspace, layers, and recipes
- snapshot tests cannot alter the operator's session file
- selecting a theme visibly changes the complete colored shell
- the footer names current, next, and previous focus destinations
- live and deterministic recovery verification passes

## M15 — Clean installed startup diagnostics

Goal: make the executable selected by the user's shell start the real Poky
workbench without terminal contamination and keep theme selection verifiable.

Capabilities:

- bounded bridge stderr capture outside the alternate screen
- actionable bridge failures with retained diagnostic context
- live installed-binary startup and theme-picker acceptance
- explicit local-development reinstall guidance for a published version

Exit criteria:

- BitBake startup notes and warnings never appear outside Ratatui panels
- bridge startup failures retain a bounded diagnostic tail
- `Ctrl+P` → `Choose theme` changes and persists the theme in a real PTY
- the shell-resolved executable matches the locally verified release binary

## M16 — Literal reference workbench

Goal: replace the earlier broad interpretation of the approved concept with a
measurable terminal-cell contract whose core visual composition matches the
reference while all values remain typed and authoritative.

Capabilities:

- strict default-theme `160x48` cell/style golden
- reference-proportioned header, mixed project Navigator, Tasks cockpit,
  Inspector stack, and stable F-key command rail
- intuitive pane focus and a directly usable persistent theme picker
- responsive degradation without panics or hidden destinations
- live Poky validation using the same rendering path as the deterministic gate

Exit criteria:

- every application-controlled cell in the canonical scene matches the reviewed golden
- the reference fixture cannot leak illustrative values into production
- every displayed function key invokes its labeled action
- theme and pane-focus behavior pass reducer, UI, and PTY interaction tests
- the complete workspace and live Poky workbench gates pass

## M17 — Responsive reference command rail

Goal: keep the reference's global function-key navigation visible throughout every
wide workbench instead of tying it to one exact terminal width and screen.

M19 `FOOTER-UI-001` intentionally supersedes the fixed presentation while
preserving every typed function-key route. The footer is now contextual and
bounded; the complete truthful function-key catalog remains in Help, and the
canonical footer geometry remains unchanged.

Capabilities:

- stable function-key rail on every screen at 130 columns or wider
- exact canonical 160×48 Tasks footer geometry remains unchanged
- contextual action footer remains available below the wide breakpoint
- installed release and PTY regression validation

Exit criteria:

- 130-, 160-, 180-, and 200-column TestBackend scenes expose the global rail
- Dashboard and Tasks both expose all ten function-key labels when wide
- compact layouts retain contextual shortcuts without horizontal panics
- the shell-resolved release binary matches the verified local artifact

## M18 — Yocto release capability compatibility

Goal: make Yoctui functionality follow the authoritative capabilities of the
connected Yocto/OpenEmbedded/BitBake environment instead of assumptions made
from the installed Yoctui binary.

Capabilities:

- authoritative typed environment identity with explicit unknown fields
- one behavior-oriented capability catalog and snapshot with evidence/reasons
- safe direct probes plus centralized conservative version fallbacks
- daemon-owned, generation-correlated capability state shared by every client
- compatible BitBake/API/utility implementations selected before typed argv
- independently probed Devtool and Recipetool subcommands/options with exact
  unavailable reasons and no cross-command authorization
- independent bitbake-layers read, create, option, add, and remove capabilities
- complete typed utility-family classification and exact command authority;
  host PATH never proves build-environment compatibility
- complete workspace action inventory, pure availability/revalidation policy,
  and shared client/runtime enforcement
- dynamic workspace/UI gating and an Environment/Compatibility inspector
- deterministic release-generation fixtures and current live multi-release evidence
- offline evidence validation plus isolated scheduled/manual fresh official
  older/latest runs with retained role-scoped diagnostics

The UI delivery is dependency-ordered: first one typed presentation projection,
then the responsive Environment/Compatibility workspace, then visible action
gating across every existing workspace and dialog. The parent UI gate closes
only after all three pass their focused and terminal-snapshot checks.

Exit criteria:

- direct capability evidence is preferred over release-number assumptions
- renderers and workspaces contain no scattered release/version policy
- older supported environments preserve safe workflows and explain unavailable ones
- unknown future releases expose only positively evidenced functionality
- latest supported stable and a materially older release have current live evidence
- exact machine-readable evidence records expire after 90 days or a relevant
  capability-contract change and never convert fixture/development runs into claims
- fixture-only tests cannot satisfy a live compatibility claim
- `./scripts/verify-compatibility.sh` independently enforces the milestone

## M19 — Next-Generation TUI Implementation and Polish

The requested milestone name used `M13`, but `M13 — Dense terminal workbench
redesign` is an existing completed and evidence-backed milestone. This work is
therefore registered as M19 without renumbering or rewriting historical tasks.
The requested parent task ID remains `M13-UI-001` for traceability.

Goal: evolve the literal workbench into a polished terminal-native IDE while
keeping Navigator / Workspace / Inspector architecture, typed actions and
effects, bounded state, capability-correlated availability, and real data as
the only rendering authority.

Capabilities:

- documented wide, medium, narrow, and below-minimum layout behavior
- reusable pane, section, status, empty/loading/unavailable, scroll, and
  responsive-column primitives
- complete semantic theme roles with high-contrast and no-color behavior
- adaptive Tasks, Logs, Jobs, Inspector, header, footer, search, palette, and
  dialog presentation
- provenance-audited bounded telemetry with honest unavailable states
- keyboard/mouse parity, reduced motion, terminal-reader text equivalents,
  PTY coverage, and explicit breakpoint tests
- semantic TestBackend snapshots, a small reviewed target-design golden set,
  and style invariants
- measured rendering budgets and evidence-backed caching only where justified
- fresh real-Poky UI evidence and current real-binary README screenshots

Exit criteria:

- every required M19 task is `DONE` with its focused evidence
- no displayed value or action is fabricated from the concept image
- all existing workspace functionality and capability gating remains reachable
- `scripts/verify-next-generation-ui.sh` independently verifies the requested
  unit, visual, golden, keymap, responsive, mouse, PTY, accessibility,
  performance, live-evidence, screenshot, Clippy, formatting, and regression
  categories
- `./scripts/verify-completion.sh` passes without weakening an older gate

## M20 — Raw BitBake Command Workbench

Goal: provide expert users with a structured, capability-correlated browser
over the BitBake CLI command surface without introducing a shell-evaluation
escape path or duplicating the embedded terminal.

Capabilities:

- a tracked Wrynose 6.0 / BitBake 2.18 reference snapshot with exact catalog
  traceability
- typed categories, command templates, descriptions, parameters, interaction
  modes, safety classes, and capability requirements
- authoritative recipe, image, target, task, and multiconfig selection with
  bounded manual entry where BitBake accepts it
- a bounded expert argument editor and exact indexed native-argv preview
- daemon-owned noninteractive jobs and daemon-owned interactive PTY sessions
- responsive category, command, help, configuration, output, history, search,
  and favorite workflows
- atomic persistent favorites and bounded command history
- dynamic availability from the connected daemon capability snapshot
- mouse, keyboard, accessibility, security, fixture, and live-BitBake evidence

Exit criteria:

- reference-only, companion-tool, pipeline, and conceptual examples are never
  misrepresented as executable Raw Mode commands
- ordinary execution never uses `sh -c`, `bash -c`, `eval`, or an equivalent
  command string
- unavailable or stale capability state fails before process or PTY creation
- closing or detaching a view does not implicitly terminate daemon-owned work
- `./scripts/verify-raw-mode.sh` and the unchanged full completion gate pass

## M21 — One-Stop Yocto Workbench Usability

Goal: turn the complete typed workbench into the most discoverable, consistent,
beautiful, and efficient terminal environment for daily Yocto work without
trading away authoritative data, safety, accessibility, or bounded behavior.

The detailed research, widget decisions, interaction contract, license policy,
phase progress, test matrix, and completion criteria live in
[`workbench-ux-roadmap.md`](workbench-ux-roadmap.md).

Capabilities:

- one typed action catalog shared by application/context menus, palette, Help,
  footer, mouse routes, keybinding preferences, and tests
- stable mnemonic defaults, scoped configurable bindings, collision detection,
  contextual discovery, predictable focus/subfocus/zoom, and common scrolling
- authoritative hierarchical progress, resource/cache meters, throbbers,
  telemetry histories, charts, accessible checkboxes, and consistent state text
- virtualized searchable logs, a safe reducer-owned multiline editor, trees,
  scroll views, variable-height lists, and dependency topology
- image-correlated package and filesystem rootfs composition with pie/bar/table/
  tree views and exact accessible fallbacks
- a first-class daemon-owned terminal/session workspace, with a measured
  `tui-term` compatibility decision that cannot weaken the typed screen boundary
- capability-aware command center, onboarding, preferences, responsive layouts,
  accessibility, performance, real-PTY tests, and supported live-Yocto evidence
- license/MSRV/source/feature review, notices, SBOM, locked dependencies, and
  `cargo deny` for every adopted third-party widget

Exit criteria:

- all 38 required M21 tasks are `DONE`
- menus, Help, palette, footer, and configured bindings cannot drift
- all visual progress and composition values are typed and text-equivalent
- terminal, editor, rootfs, log, focus, scroll, mouse, and keyboard flows pass
  deterministic and real-PTY coverage at every supported breakpoint
- every dependency has current compatible license and supply-chain evidence
- the expanded performance matrix stays below the existing 10 ms/frame ceiling
- supported live-Yocto evidence and user documentation are current
- `./scripts/verify-workbench-ux.sh` and the unchanged completion gate pass

## M22 — Concept-to-Live UI Parity

Goal: make every reviewed concept use case reproducible through the production
Yoctui renderer and demonstrably reachable in a real supported-host instance,
without treating generated artwork or broad scenario labels as implementation
evidence.

Capabilities:

- machine-checked per-scenario feature, fixture, raster, and live-evidence contracts
- a complete failed-build workspace with summary, structured diagnostics,
  correlated paused log, textual filters, and recovery actions
- canonical-width Rootfs composition with chart, exact table, accessible batch
  selection, and filesystem drill-down visible together
- a real recipe editor and focus-trapped F12 application menu composition
- live daemon-owned Terminal Sessions navigation, split, writer/read-only, and
  prefix-help evidence
- deterministic PNG rendering from exact production TestBackend cells and styles
- supported-host live captures attributed only to interactions the harness drove

Exit criteria:

- every scenario manifest gap is closed by a `DONE` owner task
- deterministic cell/style goldens and app-derived raster captures agree
- the live harness drives and verifies each claimed screen instead of inferring it
- the concept comparison report records fixture, raster, and live results separately
- `./scripts/verify-m22-concept-parity.sh` and the unchanged completion gate pass

## M23 — Integrated Devtool Editing and Shell Workflow

Goal: make the selected-recipe development loop explicit and continuous inside
Yoctui: prepare a Devtool workspace, edit metadata or source with useful
language context, build the owning recipe, and publish the change back to a
configured layer without losing terminal or job state.

Capabilities:

- a shared reducer-owned workspace editor for recipes and Devtool source trees
  with cursor motion, selection, undo/redo, search, diff/save state, line
  numbers, language detection, syntax presentation, and bounded diagnostics
- direct continuation from `devtool modify` into edit, selected-recipe build,
  `update-recipe`, and configured-layer `finish`
- user-visible Devtool workspace and `edit-recipe` session routes
- a focus-trapped launch chooser before build shell, devshell, menuconfig, or
  Devtool interactive sessions start
- embedded daemon-owned PTY and detached desktop-terminal destinations built
  from the same validated native argv and initialized environment

Exit criteria:

- no interactive terminal is spawned before the user confirms its destination
- cancelling the chooser creates neither an embedded nor detached process
- the editor identifies BitBake metadata and common source languages without
  claiming LSP authority that is not present
- recipe builds target the exact Devtool recipe and patch publication remains
  capability- and configured-layer-gated
- focused model/app/UI/CLI tests, responsive rendering, strict lint, installed
  release smoke validation, and the unchanged completion gate pass

## M24 — Real Yoctui Design Regression Gallery

Goal: keep the six supported-host M22 Yoctui screens directly reviewable under
the design documentation and prevent their provenance, membership, ordering,
dimensions, or bytes from drifting away from the live evidence bundle.

Capabilities:

- one documented gallery containing idle, active build, failed build, rootfs,
  editor/menu, and terminal-session screens
- a machine-readable capture manifest with exact source commit, binary, host,
  Yocto/BitBake, machine, target, geometry, and per-screen SHA-256 identity
- byte-for-byte linkage from every design PNG to its supported-host live raster
- regression coverage for exact membership, ordering, README links, hashes, and
  `1600x1000` dimensions

Exit criteria:

- all six real Yoctui screens render from the design README
- no fixture, production-cell raster, or concept artwork can satisfy the live
  baseline checks
- `python3 scripts/test-m22-live-design-gallery.py`, documentation checks, and
  the M22 parity gate pass

## M25 — M21 Visual Resemblance Remediation

Goal: correct production workbench geometry and scene composition so the real
executable visibly follows the six M21 concepts, and replace monochrome
semantic screenshots with color- and style-faithful PTY evidence.

Capabilities:

- two-level workbench header, M21 pane proportions, semantic title color, and
  bordered footer across all six scenes
- concept-shaped Dashboard and integrated recipe editor/F12 menu compositions
- ANSI SGR-aware live terminal composition serialized as exact cell/style data
- deterministic rasters for review followed by six fresh release-binary live
  captures from the initialized Poky environment

Exit criteria:

- all six deterministic production rasters pass reviewed geometry and semantic
  tests and materially resemble their M21 counterparts
- the live capture pipeline preserves foreground, background, and bold styles
- six current-commit live captures replace historical evidence as the visual
  regression baseline; workflow-only anchors cannot satisfy the gate

## M26 — Visible release identity

Goal: make the exact running Yoctui revision legible to operators and prevent
unversioned repository changes.

Exit criteria:

- the persistent header shows `yoctui v<workspace-version>` at every supported width
- all workspace packages and internal path constraints share one version
- CI and the completion gate reject commits that do not increment that version

## M27 — Dashboard telemetry dial fidelity

Goal: make normal-height Dashboard telemetry read as compact instruments and
materially match the M21 resource-meter concepts without sacrificing truthful
typed values or responsive fallbacks.

Exit criteria:

- CPU, RAM, and Build FS use foreground-only semicircular dials when a strip
  cell has enough width and height
- every dial retains its exact percentage and metric-specific context in text
- Unicode, ASCII, no-color, reduced-motion, short-cell fallback, and
  unavailable-state behavior remain deterministic and tested
- production scenario goldens and design rasters encode the reviewed change

## M28 — Operator shell polish

Goal: make resource monitoring and everyday shell navigation denser, quieter,
safer, and more immediately legible without importing another application's
implementation or visual identity wholesale.

Exit criteria:

- CPU, RAM, and Build FS use original history-first monitor tiles with a live
  trend field and thin threshold meter, and narrow network/disk cells keep both
  current rates and retained history visible
- Left/Right navigate Navigator, Workspace, and Inspector; Navigator tree
  expansion remains available on h/l
- unknown Navigator/task state avoids repeated question-mark markers, while
  active tasks remain left-aligned with an explicit circular activity marker
- the header highlights project/target/machine/distro and identifies Local or
  SSH access with a validated remote client IP
- theme choices use color-oriented names without changing persisted identifiers
- q/Ctrl-C and active-build c require exact, focus-trapped confirmations

## M29 — Live client state coherence

Goal: a long-running attached client never presents authority-free build work
or stale cells from an earlier terminal geometry.

Exit criteria:

- an unexpected daemon transport loss immediately retires active task rows as
  Lost and removes the build from Parsing/Running state
- the interactive client performs bounded periodic reattachment and installs
  the authoritative replacement snapshot
- terminal resize forces a complete redraw, and regression tests prove
  Navigator destinations and workspace titles are not duplicated

## M30 — Attached-client authority hardening

Goal: attaching from any shell preserves daemon-owned project/build truth and
reports the real remote client origin.

Exit criteria:

- an attached client never performs a competing local startup metadata probe
- daemon workspace identity restores canonical source/build paths even without
  a retained Workspace build event
- malformed SSH environment input falls through to another valid OpenSSH
  client-address variable
- live attachment outside the initialized shell remains Connected and Idle
  with zero false build errors

## M31 — Image build target authority

Goal: deployed output files cannot be submitted to BitBake as image recipe
targets, and failed daemon jobs retain enough target/outcome context to be
diagnosed from the CLI.

Exit criteria:

- Images `b` builds only an exact recipe-backed artifact identity
- kernel, bootloader, and other non-recipe deploy entries remain inspectable
  without changing the selected build target
- daemon job labels preserve requested targets through terminal state
- `daemon status` reports the retained target and terminal outcome
- standard Poky non-minimal image recipes pass a live BitBake no-execute probe

## M32 — Post-build package authority

Goal: a successful image build exposes generated package data immediately
without requiring Yoctui to restart.

Exit criteria:

- package inventory and detail workers bind the current validated daemon
  compatibility snapshot immediately before execution
- adapter instances created before attachment or build completion cannot retain
  an absent or stale capability generation
- every worker uses the exact current BitBake-reported, potentially
  machine-scoped `PKGDATA_DIR`
- generated `tmp/pkgdata` is distinguished from missing capability authority
- focused, workspace, Clippy, documentation, roadmap, and completion gates pass

## M33 — External-process screen restoration

Goal: returning from Neovim, another external editor, or an inherited Yocto
shell always restores a clean Yoctui frame.

Exit criteria:

- every successful alternate-screen resume schedules one full backend clear
- the clear occurs before the next rendered frame and resets Ratatui's retained
  cell buffer as well as the physical terminal
- repeated loop iterations do not perform noisy redundant clears
- focused, workspace, Clippy, documentation, roadmap, and completion gates pass

## M34 — Rootfs pkgdata and selection viewport hardening

Goal: keep current-release image composition available for large generated
package records and keep the operator's selected row visible in every clipped
collection.

Exit criteria:

- generated runtime pkgdata is streamed under explicit file, line, aggregate,
  timeout, and cancellation bounds
- scoped installed-size and file-map fields retain exact package evidence
- an isolated resource-limit result is Partial rather than a screen failure
- workspace collections and modal pickers follow the selected identity through
  the bottom row at all supported geometries
- focused, live-rootfs, workspace, Clippy, documentation, roadmap, and
  completion gates pass

## M35 — Segmented workbench meters

Goal: use one compact btop-inspired but original square-dot visual language for
resource utilization, build progress, task progress, and running activity.

Exit criteria:

- CPU, RAM, and build-filesystem tracks use threshold-colored square segments
- overall build, task, and reusable semantic progress use the same filled and
  remaining square-dot vocabulary
- indeterminate running and waiting markers contain no circular spinner glyphs
- Unicode, ASCII, no-color, and reduced-motion modes preserve exact text
- exact cell goldens and all six deterministic production screenshots prove
  the real renderer, not a separate mockup

## M36 — Bottom-edge selection visibility

Goal: every scrollable menu keeps its highlighted selection visible through
the final row, including inventories larger than the rendered table.

Exit criteria:

- Compatibility capabilities use a selection-aware viewport sized from the
  table's real border/header-adjusted capacity
- the capability title reports the exact visible row range and filtered total
- a full capability inventory regression reaches the last stable identity and
  proves that the first page has scrolled away
- the narrow context menu regression proves the last row remains both visible
  and highlighted
- focused, workspace, Clippy, documentation, roadmap, and completion gates pass

## M37 — Viewport chrome polish

Goal: make every important long menu communicate its current position and
remaining scroll directions without reducing the content area.

Exit criteria:

- Navigator chrome shows the selected visible row and available direction
- application/context menus show selection, total, exact visible range, and
  available direction whenever their catalog is clipped
- command-palette and Compatibility inventory titles use the same cue
- cues disappear for collections that fit and remain truthful at the first,
  middle, and final viewport
- Unicode, ASCII/no-color, responsive, exact-golden, workspace, Clippy,
  documentation, roadmap, and completion gates pass

## M38 — Braille activity and Rootfs chart polish

Goal: use the admitted third-party widget vocabularies consistently for active
work and image composition while retaining exact accessible evidence.

Exit criteria:

- every animated Unicode activity marker uses
  `throbber-widgets-tui::BRAILLE_EIGHT_DOUBLE` through one stateless adapter
- ASCII, reduced-motion, and terminal lifecycle fallbacks remain explicit
- wide color/Unicode Rootfs composition uses `tui-piechart` at Braille
  resolution
- exact byte/percentage tables and narrow/no-color/ASCII fallbacks remain
  authoritative and visible
- focused, exact-golden, workspace, Clippy, documentation, roadmap, and
  completion gates pass

## M39 — Embedded image console

Goal: boot the selected image with QEMU or connect to an already-running target
over SSH without leaving the daemon-owned `tui-term` terminal workbench.

Exit criteria:

- Images `T` opens one focus-trapped Image Console dialog with explicit “Boot
  with QEMU” and “Connect over SSH” modes
- QEMU binds the exact selected rootfs/Wic artifact and inspected `runqemu`
  executable, forces `nographic` plus `serialstdio`, and starts in a daemon PTY
- SSH requires explicit host/user/port and an optional absolute identity file,
  keeps OpenSSH host-key verification enabled, stores no password, and starts
  no supplied remote command
- both session kinds are preserved across model, protocol, daemon persistence,
  terminal summaries, `tui-term` rendering, writer leases, and reconnect
- opening/cancelling is no-spawn; missing/stale capability and malformed input
  fail closed with an exact visible reason
- focused, workspace, Clippy, documentation, roadmap, and completion gates pass

## M40 — Layers tree widget integration

Goal: render the bounded lazy Layers filesystem with the admitted
`tui-tree-widget` vocabulary without giving a renderer ownership of Yoctui
state.

Exit criteria:

- nested loaded directories and files render through `tui-tree-widget` 0.24.1
- stable path selection, lazy expansion, filtering, Git decoration, and
  viewport intent remain model-owned
- a fresh transient `TreeState` is derived for each frame and no widget event
  helper handles keyboard or mouse input
- Unicode, ASCII/no-color, malformed identity, and bottom-edge selection paths
  are regression-tested
- notices, SBOM, `cargo deny`, focused UI, workspace, Clippy, documentation,
  roadmap, and completion gates pass

## M41 — Hierarchical key routing

Goal: make Layers and Navigator hierarchy keys reach their typed owners without
being shadowed by pane focus or passive notifications.

Exit criteria:

- configured Layers opens with `Enter`, `Right`, or `l`
- the open Layers tree expands with `Right`/`l`, collapses or selects its parent
  with `Left`/`h`, and toggles directories or edits files with `Enter`
- Navigator `Right`/`Left` expands/collapses groups, while `Tab`/`Shift+Tab`
  remain the unambiguous pane-focus controls
- passive notifications do not consume `Enter`; actionable failure
  notifications and `Esc` retain their documented routes
- focused routing, reducer, UI, workspace, Clippy, documentation, roadmap, and
  completion gates pass

## M42 — Unified workbench parity

Goal: make startup, Dashboard, content previews, rootfs visualization, and the
edit-to-build loop match the reviewed workbench interaction model.

Exit criteria:

- daemon startup and indeterminate work share Braille Eight Double activity
- startup focuses Dashboard in Navigator and focus skips passive panes
- Dashboard uses the Tasks cockpit with honest idle/unknown progress
- Layers and Recipes integrate scrollable previews without duplicate inspectors
- Vim-saved source retains a visible diff and can immediately build via Ctrl+B
- Layers supports PageUp/PageDown and rootfs prioritizes the Braille pie chart
- focused, exact-golden, workspace, Clippy, documentation, roadmap, and
  completion gates pass in version 0.1.19

## M43 — Dashboard focus and preview density

Goal: remove the last passive Dashboard focus target and spend sparse Layers
browser space on source inspection.

Exit criteria:

- Dashboard remains Navigator-only with idle, active, completed, and retained
  build evidence
- direct pane focus, Dashboard navigation, and modal restoration cannot land on
  its read-only Tasks cockpit
- collapsed Navigator roots retain their selected group and reopen through
  Right, Enter, or another click
- the Layers tree column follows useful label width within responsive bounds
- unused browser width expands the scrollable file preview
- guidance and failure notifications use a clearly visible dismissible popup
  without becoming a focus trap
- focused, workspace, Clippy, documentation, and roadmap checks pass in
  version 0.1.20

## M44 — Live navigation, logs, and cancellation

Goal: remove the regressions that strand collapsed Navigator roots, erase
task-log context at daemon IPC, and leave accepted cancellation pending.

Exit criteria:

- every collapsed Navigator root remains reachable after moving to another root
- Right, `l`, and Enter reopen the reselected collapsed root
- daemon log snapshots and events preserve recipe, task, source path, and build
  correlation for Dashboard and Logs
- Logs opens on its scrollable workspace and supports the complete bounded
  vertical navigation vocabulary
- accepted cancellation has a bounded forced-server terminal fallback
- focused, protocol, workspace, Clippy, documentation, and roadmap checks pass
  in version 0.1.21

## M45 — Live build projection correctness

Goal: make the build phase, selected-task log, and shared job history reflect
the daemon's live BitBake authority.

Exit criteria:

- task activity transitions Parsing to Running and prevents late parse regressions
- real BitBake `taskpid` log records correlate with the selected recipe task
- daemon-owned jobs populate Dashboard and Job History without duplicate rows
- focused, workspace, Clippy, documentation, and roadmap checks pass in
  version 0.1.22

## M46 — Low-Overhead / Build-Saturation Responsiveness

Goal: keep Yoctui interactive and correct while BitBake saturates the host,
with a combined daemon plus one attached client steady-state overhead of at
most 1% of one logical CPU in the defined normal-operation baseline.

Exit criteria:

- exact one-logical-CPU accounting and per-scenario CPU/latency/resource
  thresholds are documented before optimization
- reproducible pre-optimization baselines and workload-specific flamegraphs
  identify actual hot paths and wakeup sources
- idle loops block, rendering is dirty/event-driven, animations and telemetry
  are bounded, and identical frames are not redrawn
- high-rate logs and task progress coalesce without losing failures,
  cancellation, disconnects, terminal outcomes, warnings, or errors
- slow clients cannot block the daemon or BitBake connection; bounded
  per-client pressure and reconnect behavior are observable
- deterministic full-CPU saturation and BitBake-like event-flood fixtures gate
  input, render, IPC, cancellation, PTY, and memory behavior without network
- supported real-Poky saturation evidence is distinct from fixture evidence
- the dedicated performance verifier and independent completion checks pass

Progress: the pre-optimization evidence, deterministic saturation/event-flood
fixtures, idle event-loop optimization, dirty-render scheduler, and bounded
animation clock are complete. Kernel-backed daemon listener readiness reduced
the focused idle result from roughly 867 to below 35 voluntary context
switches/s; unchanged polls do not redraw, state bursts coalesce, and only
visible indeterminate Dashboard/Tasks activity advances at 4 Hz. Client host
telemetry now uses 1/0.1 Hz visible/background tiers, while daemon health pauses
without clients and uses 0.2/1 Hz idle/active tiers. Logs normalize once,
ordered IPC bursts reduce in bounded batches, and render metadata comes from
one filtered pass. Task bursts now reduce with lifecycle barriers and
stable-identity progress coalescing; cached row ordering avoids repeated
filter/sort work, and active/completed bounds retain terminal failures. IPC
measurement found full-snapshot JSON serialization at 34.09% self CPU. A
conservative size ledger, bounded in-place build/log reduction, incremental
32-event catch-up, and shared fanout encoding now keep the measured client at
33.58 KiB/s with zero replacement snapshots. Bounded priority-aware client and
supervisor queues are complete: a 4,000-event/s run retained every sentinel for
the healthy client while isolating a non-reader and preserving a fresh attach.
BitBake terminal publication now precedes cleanup, silent scheduler delay is
not a disconnect, real EOF remains authoritative, and cancellation survives an
all-CPU load while hung cleanup stays off the critical path. The Tokio audit
reduced idle runtime workers from eight to two and stable process threads from
nine to three, while a deterministic all-CPU test proves a blocked worker does
not starve reactor work. The scheduler audit keeps inherited nice 0 as the
required path: nice 5 was worse, unprivileged nice -5 was denied, and a user
service at CPUWeight 200 provided no material median-p95 benefit. Optional CPU
affinity also provided no benefit on the reference SMT host: the inherited full
set outperformed both pinned layouts while remaining responsive with every CPU
busy. The coexistence audit then measured 0.6223 ms median p95 wake latency at
one worker per logical CPU and 1.0377 ms at two workers per CPU while host CPU
stayed near 99%. `yoctui inspect` now reports a typed, read-only
`BB_NUMBER_THREADS`/`PARALLEL_MAKE`/load diagnostic; examples require review and
never change build configuration. The real-PTY input audit collected 100
post-warmup observations per path at 99.75% host CPU: keyboard-to-model,
keyboard-to-frame, and mouse-to-selection p95 were 1.974, 4.336, and 4.440 ms.
The saturated production IPC audit is also complete: 100 monotonic samples per
path cover timestamped daemon events, correlated command results, and live
BitBake cancellation acknowledgements while every affinity CPU is runnable.
At 99.26% host CPU their p95 values were 3.262, 0.229, and 1.359 ms
respectively. Exact release measurements and source/binary hashes are retained
by the offline `--latency` continuity gate. The release CPU gate then exposed
and removed a separate attached-idle one-millisecond daemon loop: one shared
listener/client readiness wait reduced the full 60-sample reference result to
0.0000% idle daemon, 0.0624% idle client, and 0.1456% combined CPU of one
logical CPU. Exact process identities, raw samples, host/binary/source hashes,
and independently recalculated thresholds are retained. The saturated
interaction aggregate now passes as well: it composes the real-PTY input probe,
production BitBake/IPC path, and full-affinity load. Keyboard-to-frame and
mouse-to-selection p95 remain 4.336/4.440 ms; refreshed daemon-event and
cancellation-ack p95 are 3.262/1.359 ms. Delayed backend events, explicit EOF,
cancellation, a 0.119 ms acknowledged detach, and a fresh attach all complete
while saturation remains active. The default IPC continuity gate now validates
hashed flood and saturation evidence and reruns the bounded production path:
all correctness-critical sentinels remain ordered, the healthy client never
resynchronizes, a non-reader is isolated, and detach/reconnect remain available
under load. Bounded-memory endurance is next.
The bounded-memory gate now passes its 30-minute release run: daemon/client RSS
growth was 1.26 MiB/256 KiB, final 20-minute slopes were 0/1.1 KiB per minute,
threads stayed 3/1, and correctness/continuity survived the full 4,000-event/s
stream.
Supported real-Poky saturation evidence now passes against Poky 6.0.2. A
daemon-owned `linux-yocto:do_compile` kept the reference host at 99.6836% CPU
for 120 measured seconds while the release daemon/client used
0.4297%/0.4496%, or 0.9207% combined of one logical CPU. Adaptive saturation
presentation reduces cosmetic full-frame work to 1 Hz above 90% host CPU while
input remains immediate; 100 probes measured 5.4191 ms p95. Cancellation,
fresh attach, backend continuity, and bounded IPC pressure all passed with
exact raw and hashed evidence. Machine-readable regression aggregation is
complete. The deterministic record binds eight retained evidence sources and
collects CPU, latency, wakeup, render, pressure, and memory data in one compact
schema. Twenty-two controlled thresholds remain hard gates and seven continuity
and ordering checks pass; exact regeneration prevents stale or hand-edited
summaries, while diagnostic trends tolerate tiny uncontrolled variance. CI
integration is complete. Push/PR CI runs the bounded idle-loop, render,
coalescing, full-affinity saturation, and IPC-backpressure checks. Weekly/manual
CI repeats the release CPU and 30-minute memory paths while validating retained
profiles and real-Poky evidence; explicitly enabled labeled self-hosted runners
can capture fresh real-Poky evidence. All performance jobs retain diagnostics
on failure. Low-overhead architecture and operator documentation is complete:
the performance guide, architecture, and UI specification share the exact
accounting, cadence, telemetry, backpressure, liveness, safe tuning, profiling,
and evidence rules. The independent M46 parent gate now checks every child,
dynamic release gate, profile, and recorded real-Poky artifact, while the
repository completion script repeats the required quality and performance
boundaries. M46 is complete in v0.1.51.

## M52 — Yocto logs, image consoles, and udev inventory

Integrate tui-logger presentation while preserving bounded typed Yocto log
authority and navigation/correlation controls. Verify existing tui-term SSH and
runqemu paths. Add image-owned udev rules with bounded offline preview and
precedence/masking evidence. Dependency, visual, performance, and completion
gates remain required.

The three feature children are implemented (v0.1.58–0.1.60). The integrated
v0.1.61 candidate and profile-guided ASCII optimization pass functional and
rendering checks, with real-build combined CPU at 0.9777% of one logical CPU.
M52 release completion is BLOCKED by strict Memcheck findings for fixed
tui-logger/Jiff process-lifetime caches, pending upstream cleanup support or
an explicitly approved bounded exception. No verification rule was weakened.

## M51 — Workbench integration release

Complete in v0.1.57: the M47–M50 feature histories coexist with completed M46
scheduling, IPC, and performance gates. Navigator identity, mouse effects,
milestone numbers, and reviewed visual evidence are reconciled. Task-specific
workspace and performance checks pass; a measured editor rendering regression
is fixed with exact dirty/diff text comparisons while preserving revision
hashes. Full completion verification must independently pass on the clean
merge candidate before the combined release advances master.

## M47 — Kernel configuration and device trees

Goal: make the active kernel provider directly inspectable and configurable
without leaving Yoctui.

Exit criteria:

- Kernel is a dedicated Content destination resolved through `virtual/kernel`
- provider-reported menuconfig runs in the persistent PTY
- `.config`, DTS/DTSI, and DTB/DTBO artifacts are browsable from authoritative roots
- exact confirmed `dtc` compile/decompile operations refuse overwrites
- bounded scanning, focused tests, workspace checks, and documentation pass in version 0.1.47

## M48 — U-Boot and BIOS/UEFI workbench

Goal: expose the active image's boot firmware with the same direct
configuration and device-tree workflow as the kernel.

Exit criteria:

- U-Boot / BIOS is a separate Content destination
- image-scoped bootloader and EFI variables plus recipe metadata select the provider
- the detected target's advertised menuconfig task runs in the persistent PTY
- `.config`, DTS/DTSI, and DTB/DTBO artifacts use the shared bounded explorer
- focused tests, workspace checks, and documentation pass in version 0.1.48

## M49 — Offline rootfs system explorer

M49 adds offline exploration of BitBake's staged image root. Images can open
the exact `IMAGE_ROOTFS`, list and edit systemd service and system-bus
activation files, map offline configuration relationships, and keep preview
scrolling separate from global search. Mouse navigation now executes the same
Packages-loading effects as keyboard navigation.

## M50 — Overview insights and dependency exploration

Goal: place build, cache, image, provenance, package, supply-chain, and disk
visualizations beside Dashboard without creating a second data authority.

Exit criteria:

- Overview contains one responsive Insights workspace with eight directly
  selectable visualizations
- build timeline and critical path use retained task timestamps and dependency
  identities; rebuild causes use typed signature differences
- sstate and downloads show observed setscene/fetch outcomes and configured
  cache paths without guessed sizes
- rootfs installed bytes and same-target build deltas, metadata provenance,
  runtime package dependencies, and disk telemetry have bounded visual
  projections and explicit empty states
- Security imports and explores SPDX, CycloneDX JSON, and legacy Yocto image
  manifests, with exact fallback limitations
- the `tui-piechart` rootfs surface preserves its exact table and exploration
  panes at every responsive breakpoint
- process-backend `bitbake -g` task nodes are reachable from the selected recipe
  root and retain cycle-safe reverse/path exploration
- focused parser, model, UI, workspace, Clippy, documentation, and roadmap
  checks pass in version 0.1.47

Progress: complete on `feature/overview-visualizations`.

## M53 — Guided environment path setup

Add a local directory browser alongside plain manual Source/Build/Script
fields, explicit save/cancel and the existing verify-before-build boundary.
Bounded filesystem fixtures and responsive rendering tests cover disconnected
startup without requiring a daemon or executing Poky initialization.

Progress: complete in v0.1.62, including the disconnected real-PTY startup
regression. The independent M52 release-policy blocker remains open.

## M54 — Operator README and crates.io release

Document implemented features and practical workflows without promotional
copy. Install the requested candidate locally without restarting active jobs.
Publish the six public crates in dependency order after independent release
and package verification; M52's memory-policy blocker still applies.

Progress: README, operator-coverage tests, package verification and requested
local installation complete in v0.1.63. Publication is a post-gate delivery
task, not a circular prerequisite for product completion; it remains blocked
on the independent M52 memory-policy decision. No crates were uploaded.

The user approved the reviewed bounded upstream-cache exception on 2026-09-06.
M52 validation passed for v0.1.64, including fresh Memcheck, rendering,
flamegraph, idle CPU and real-Poky saturation measurements. M54 publication is
complete: full completion passed before upload, all six public crates are
published as v0.1.64, and registry checksum/install/real-PTY verification passed.
The optimized release is installed locally with a backup; the daemon was not
restarted.

## M55 — Compact resource visibility

Keep four-row CPU/RAM/filesystem square-dot meters visible on short Dashboard
and Tasks workspaces down to the supported minimum. Preserve typed values,
unknown states, task selection, log access, accessibility, and the existing
full-size telemetry tier. Rendering-only changes must not add polling or focus.

Progress: COMPACT-TELEMETRY-001 DONE in the v0.1.65 candidate; full workspace,
Clippy, bridge, documentation and reviewed UI regressions pass. Not yet installed
or released. Package availability was diagnosed
separately as stale authority in the old v0.1.21 daemon. The user's subsequent
shutdown request was completed through acknowledged cancellation and daemon stop.

## M56 — OpenBMC live integration

After M55, provision a separate OpenBMC checkout and supported machine build
directory without modifying Poky. Record the revision, machine, initialization
command, storage budget and prerequisites. Exercise a real image build through
Yoctui and inspect package/rootfs availability, task/log lifecycles and terminal
outcomes. Register atomic regression fixes for observed defects; fixture tests
alone do not establish OpenBMC compatibility. Storage is currently constrained
and Poky generated output must not be removed without approval.
Progress: environment, direct capabilities, responsive startup, bounded inventory,
CLI submission, symlinked-tool discovery, host Python and initial attachment
tasks are DONE through v0.1.73. Package mapping and attached rootfs-source
repairs passed real UI rechecks in v0.1.85/v0.1.86; OPENBMC-LIVE-001 is DONE
in v0.1.87. RELEASE-PERF-REFRESH-001 retains the source-bound release follow-up.
Its v88 CPU finding is separately registered as RELEASE-DAEMON-CPU-001.
OPENBMC-SNAPSHOT-PROGRESS-001 is DONE in v0.1.76: typed aggregate counters survive
task-event compaction, with baseline tests and real attached-counter verification.
DAEMON-JOB-IDENTITY-001 is DONE in v0.1.78: shared checked allocation preserves
recovered history and cross-supervisor identity. All baseline checks and isolated
real-daemon recovery/fake-bridge cancellation regressions pass. The real image
completed on the preserved v0.1.76 daemon without interruption for deployment.
OPENBMC-ATTACH-TIMING-001 is DONE in v0.1.80: observed timestamps survive
reattachment and terminal durations freeze. OPENBMC-TASK-IDENTITY-001 is DONE
in v0.1.81: bounded initialized PN metadata replaces filename-derived ghost
queue identities while unresolved events preserve only aggregate statistics.
Both have full baseline and focused regression coverage; live new-daemon
rechecks passed in v0.1.84 after successful natural image completion.
The image retry reached 4,953/6,812 tasks without a terminal result before the
September 8 host OOM and user-session shutdown. A third attempt was accepted
through the preserved v0.1.73 candidate with OpenBMC-only concurrency reduced
to two BitBake tasks and two compiler jobs. The fifth attempt succeeded;
artifact/rootfs checks passed, while production UI inspection registered the
two exact metadata integration defects described above.
Root had about 46 GiB free at this restart, subject to disk and memory monitoring.
The existing 15 GiB scheduling-stop and 8 GiB halt margins remain configured.
See [live integration preflight](testing/openbmc.md).

## M57 — README visual identity

Adapt the supplied circuit-style wordmark into a compact header with clickable,
accurate badges and navigation, preserving the operator guide and screenshots.
Do not bake release versions, coverage percentages or nonexistent community
links into the artwork. Contract: [README header](design/readme-header.md).
README-HEADER-001 is DONE in v0.1.74: static header/link/asset checks, version,
formatting, locked metadata and raster/gallery verification pass. No runtime
source changed and no release is certified. OpenBMC validation resumes.

The user-requested README production gallery is DONE in v0.1.91 under
README-SCREENSHOTS-001. It adds current deterministic screenshots for Kernel
and U-Boot device trees, both menuconfig sessions, the rootfs package pie, and
the established representative screens. Fixture and live-capture authority
remain explicitly distinct. RELEASE-FIXTURE-READY-001 resumes after the
higher-priority documentation request is committed and pushed.

## M58 — Shared utilities and library decomposition

REF09-UTILS, REF09-MODULES and REF09-VERIFY consolidate portable helpers,
repair helper bugs, split lib.rs files above 1000 lines by responsibility,
and deliver baseline-verified commits with coherent package versions.

## M59 — Concept layout implementation

CONCEPT-SHELL restores the shell/dashboard, CONCEPT-DETAIL aligns the remaining
five scenes, and CONCEPT-VERIFY reviews six generated PNGs and baseline checks.

All three M59 tasks are DONE in v0.1.106. All six production images are reviewed;
the full workspace baseline and deterministic raster checks pass. See the
[concept layout review](design/concept-layout-review.md) for terminal adaptations
and the distinction between fixture images and retained live evidence.

## M60 — Startup responsiveness and utility consolidation

PERF-STARTUP-001 keeps the daemon and client at normal priority while automatic
startup recipe discovery and its metadata descendants use background priority.
REF12-UTILS consolidates exact path, text, identifier, bounded-insertion and
spawn-retry rules in `yoctui-utils`, including portable absolute-path handling.
PERF-VERIFY-001 requires release-profile idle CPU below the existing 1% combined
contract, a real initialized-workspace priority observation, the full baseline,
and delivery of version 0.1.107.

## M61 — Navigator-first actionable focus

FOCUS-NAVIGATOR-001 keeps focus on Navigator at startup and after destination
navigation. Tab and Shift+Tab are the only keyboard routes between panes, and
pane traversal omits informational panes without user-controlled content.

DONE in v0.1.108: the shared model policy now drives keyboard, mouse, menus,
responsive switching and footer hints. Deterministic concept captures show
Navigator focus on passive screens and an explicit Tab route only for
actionable Workspaces.

## M62 — Native menuconfig presentation

MENUCONFIG-NCURSES-001 gives kernel and U-Boot menuconfig sessions the full
Terminal Sessions workspace beside Navigator, removes the passive Inspector
for the selected menuconfig session, synchronizes the writer-owned PTY to the
visible cell area, and preserves the native ncurses colors and attributes.
Deterministic gallery images exercise the same typed terminal-cell renderer.

DONE in v0.1.109. The shared renderer and pointer topology, writer-only
deduplicated resize path, terminal environment normalization, focused tests,
full baseline, and both reviewed menuconfig gallery images pass.

## M63 — Native menuconfig README

README-MENUCONFIG-002 documents that kernel and U-Boot menuconfig retain their
original ncurses layout, colors and controls in the embedded terminal. It also
records the expanded workspace, omitted passive Inspector and visible-pane PTY
resizing introduced in v0.1.109.

DONE in v0.1.110. The README descriptions, version-bearing cell goldens,
production rasters and provenance manifests are synchronized and verified.

## M64 — Concept value review and telemetry fidelity

METER-CONCEPT-001 replaces the Dashboard's heavy multi-radius resource bands
with thin semicircular instruments matching the reviewed reference. The three
available cards retain typed values, centered percentage and capacity context;
the fourth remains explicitly unavailable until the backend supplies an
authoritative sstate-reuse ratio.

The six-image value review keeps the editor and focus-trapped F12 application
menu as the first future concept investment. Its simultaneous recipe tree,
large source buffer, validation/diff region and stable menu groups would improve
an end-to-end editing workflow. Active Tasks and Failed Errors remain the next
operational polish targets because their correlated progress, logs, search and
recovery actions shorten build diagnosis. Rootfs Composition and Terminal
Sessions already deliver most of their concept value and should change only
when operator evidence exposes a concrete gap. Dashboard sample values,
timestamps, usernames, paths and decorative activity remain concept-only.

DONE in v0.1.111. Focused and full workspace tests, strict Clippy, bridge,
documentation, version/layout, roadmap, concept integrity and all deterministic
raster checks pass.

## M65 — Device-tree editor and compiler

DEVICE-TREE-EDITOR-001 adds a Device Tree language to the shared viewer/editor
and a typed `dtc` compile-options dialog for the Kernel and U-Boot workbenches.
It keeps authoritative discovery, collision refusal, exact argv preview and
daemon-owned PTY execution while exposing symbol generation, sorting, padding
and reserve-map capacity.

DONE in v0.1.112. The shared model, app routing, responsive dialog, source
renderer, documentation and full verification baseline pass. The host used for
release verification did not provide `dtc`; exact execution handoff is covered
through the daemon terminal effect and existing real-PTY tests.

## M66 — Device-tree evidence, hardening and profiling documentation

README-DTC-SCREENS-001 adds production-renderer screenshots for the shared DTS
editor and typed compile-options dialog. DEVICE-TREE-HARDEN-001 then audits the
new path and fixes concrete safety or correctness defects with failed-first
regressions. README-FLAMEGRAPH-001 restores the checked real-perf Flamegraph and
its validated summary to the landing page without presenting historical
evidence as a current source-bound measurement.

README-DTC-SCREENS-001 is DONE in v0.1.113. DEVICE-TREE-HARDEN-001 is DONE in
v0.1.114 after failed-first coverage repaired root containment, dangling-link
collision detection and the final launch recheck. README-FLAMEGRAPH-001 is DONE
in v0.1.115 with a summary-bound historical report and validated reproduction
path. All 732 registry tasks are DONE.

## M67 — Fresh setup, responsive daemon readiness and content search

User-requested atomic queue: SEARCH-CONTENT-001, FRESH-CLONE-001,
DAEMON-STARTUP-002. Regressions and baseline checks gate each implementation.

M67 implementation is complete: fresh build-directory initialization,
nonblocking compatibility discovery, content-only search and matching Help are
verified. The controlled three-second startup probe no longer gates daemon
readiness (3.246 s before, 116 ms after in the debug fixture). This does not
replace source-bound live-Yocto release performance evidence.

Release follow-up M67-LIVE-EVIDENCE-001 is BLOCKED on new current-source live
performance evidence. The retained manifest has 49 source digest mismatches;
M67 behavior/regression verification is complete, while the repository-wide
release completion gate remains unpassed. No unrelated eligible task remains.

## M68 — Responsive workflows, Git and concept UI repair

The user request prioritizes clone/progress, background cancellation, source Git
status and GitUI, concept menus, arrow/Escape focus, clean meters and a readable
Braille pie. Ten atomic tasks end with updated front README screens and a
non-force push to master. M67 live performance evidence remains a separate
blocked prerequisite and does not prevent implementing or pushing these fixes.

## M75 — Rapid live-workbench bug corrections

DEVTOOL-MENU-DAEMON-001 and RAW-RECIPE-REPEAT-001 are DONE in v0.1.225.
Devtool status now uses daemon-owned initialized environment authority, F12
Devtool actions have distinct executable routes and unavailable actions report
their reason after closing the menu. Raw recipe arguments have an in-place
searchable picker, wrapped field traversal and exact repeat-request correlation.
Focused model, app, CLI, daemon and TestBackend checks pass; the user continues
manual bug testing before requesting the full suite.

YOCTO-UTILITY-MENUS-001 is DONE in v0.1.226. Typed F12 Tools forms cover every
configuration-fragment operation plus the requested bitbake-layers inventory,
recipe-provider, overlay, and layer-creation commands. Current capability
authority gates exact shell-free argv previews and daemon-owned terminal
execution. Focused checks and strict Clippy pass; the full suite remains
deferred for the user's rapid manual bug pass.

BITBAKE-LAYERS-ALL-001 is DONE in v0.1.227. F12 Tools covers all 14 subcommands
and every command-specific option advertised by the active BitBake 2.19
bitbake-layers utility. Typed validation, per-command capability and option
probes, exact argv previews, and daemon-owned execution remain enforced.
Focused checks and strict Clippy pass; the full suite remains deferred for the
user's rapid manual bug pass.

DEVTOOL-ALL-MENU-001 is DONE in v0.1.228. F12 has a dedicated Devtool group
covering all 25 initialized-environment subcommands and every documented option.
Typed validation, exact argv previews, per-command compatibility authority, and
daemon-owned terminal execution pass focused checks and strict Clippy; the full
suite remains deferred for the user's rapid manual bug pass.

## M76 — Recipe-centered Devtool Workspace

Goal: turn the existing Devtool primitives into one first-class workspace where
an operator can select a recipe, create its workspace, edit and build it, deploy
the built install tree to a running target through Devtool's SSH/SCP transport,
and publish patches into a configured layer without changing screens.

The work is split into DEVTOOL-WORKSPACE-SURFACE-001,
DEVTOOL-WORKSPACE-LOOP-001, DEVTOOL-WORKSPACE-PATCH-001, and
DEVTOOL-WORKSPACE-RELEASE-001. The implementation reuses authoritative recipe
identity, Devtool status, editor, build, terminal, and daemon job owners; it adds
only the first-class projection and the missing configured-layer patch plan.

M76 is complete in v0.1.229. The dedicated workspace supports recipe selection,
modify/edit, exact recipe builds, workspace shell and GitUI, Devtool SSH/SCP
deployment to a running target, and previewed patch installation into a
configured layer. Focused tests, version policy, formatting, and strict Clippy
pass; the full workspace suite remains deferred for the user's manual bug pass.

## M77 — Devtool editor navigation and operational header

Goal: make the integrated Devtool source editor usable for complete source
trees and long files, add file/workspace/global search scopes, present complete
typed workspace Git state with direct GitUI access, distinguish editor panes,
and move the local clock plus visible daemon status into persistent Header
chrome.

The work is split into DEVTOOL-EDITOR-VIEWPORT-001,
DEVTOOL-EDITOR-SEARCH-001, DEVTOOL-EDITOR-GIT-001, HEADER-STATUS-001, and
DEVTOOL-EDITOR-RELEASE-001. Each task retains the existing model/app/UI/CLI
ownership boundaries and the user's focused-test release workflow.

M77 is complete in v0.1.230. The integrated editor traverses the complete
bounded workspace inventory and long documents, retains Vim-style editing,
supports selected-file and workspace search, shows complete repository
tracking with direct GitUI access, and uses distinct pane chrome. Persistent
Header chrome now shows local `HH:MM` time and a visible current daemon message
or health fallback. Focused checks, formatting, strict Clippy, and roadmap
validation pass; the optimized release is installed and the initialized
Romulus daemon was restarted. The full workspace suite remains deferred for
the user's manual bug pass.

## M78 — Raw command catalog return

RAW-CATALOG-RETURN-001 is DONE in v0.1.231. A Raw execution no longer preserves
its form and preview beneath the output screen: closing it returns directly to
the selected command catalog. The Navigator's `Tools → Raw Mode` destination
also closes that execution view and detaches an attached nonterminal daemon job
without cancellation. Focused model/app/UI checks, strict Clippy, formatting,
version policy, and roadmap validation pass; the full suite remains deferred
for the user's manual bug pass.

## M79 — Build error investigation and resolved history

Goal: make active OpenBMC build failures and saved build failures available in
one Errors workspace, open the exact task log without leaving the application,
and safely remove saved failures after a newer matching build proves they are
resolved.

ERRORS-HISTORY-VIEWER-001 and ERRORS-RESOLVED-CLEANUP-001 are complete in the
v0.1.232 series. ERRORS-WORKSPACE-RELEASE-001 packages the focused validation
and optimized binary. M79 is complete; the full workspace suite remains
deferred for the user's manual bug pass.

## M80 — Platform inspection and embedded menuconfig reliability

Goal: make lazy Kernel and U-Boot provider inspection work after a client
restart from a plain terminal, and prevent embedded menuconfig retries or
parallel platform sessions from breaking one another's custom-terminal
handoff.

PLATFORM-INSPECTION-ENV-001 reconstructs the exact selected Yocto environment
before starting the client-owned metadata bridge.
MENUCONFIG-RELAY-ISOLATION-001 gives every relay its own private runtime socket.
PTY-SESSION-ID-RECOVERY-001 ensures a restarted daemon allocates new PTY
identities above recovered terminal history so the requesting platform
workspace can bind the exact new session.
PLATFORM-PTY-RELEASE-001 is complete in v0.1.233. Live Romulus validation from
an unsourced client resolved the Kernel provider, allocated a new terminal ID
above recovered history, attached the embedded viewer, rendered Linux 6.18.49
menuconfig inside Yoctui, and returned cleanly after termination. The full
workspace suite remains deferred for the user's manual bug pass.

## M81 — Persistent Hardware library and embedded viewer

Goal: add a build-independent Hardware workspace for board, SoC, memory,
peripheral, sensor, and other documents; import supported PDFs, schematics, SVG,
and raster files through a local browser; and inspect them inside Yoctui with
page navigation, zoom, pan, and search across restarts and host reboots.

The work is split into HARDWARE-MODEL-001 for the typed library/viewer state,
HARDWARE-PERSISTENCE-001 for validated browsing, conversion, and atomic session
storage, HARDWARE-UI-001 for Navigator/application-menu access and the maximum
body viewer, and HARDWARE-RELEASE-001 for the versioned optimized release.

M81 is complete in v0.1.234. Hardware is available from Navigator and F12,
persists its categorized document library, and opens PDF, KiCad, SVG, and raster
documents in the full-body embedded viewer with page, zoom, pan, and search
controls. Focused tests, strict workspace Clippy, version policy, formatting, and
roadmap validation pass; the optimized binary is installed and the initialized
Romulus daemon was restarted. The full workspace suite remains deferred for the
user's manual feature pass.

## M82 — Embedded menuconfig PTY isolation

Goal: keep BitBake's Knotty progress terminal from overwriting or competing
for input with the Kernel and U-Boot ncurses menuconfig screen hosted by the
same relay.

MENUCONFIG-PTY-ISOLATION-001 isolates the outer BitBake client's standard
streams while preserving the validated wrapper handoff and terminal outcome.
MENUCONFIG-PTY-RELEASE-001 is complete in v0.1.235. A live 210x50 Romulus
session confirmed the outer BitBake client used null standard streams while
`mconf` exclusively owned the embedded PTY; menuconfig rendered without the
Knotty 99% footer, accepted navigation, exited normally, and completed
`do_menuconfig`. The optimized binary is installed and the initialized daemon
was restarted. The full workspace suite remains deferred until requested.

## M83 — Ncurses DEC graphics emulation

Goal: render ncurses interfaces with their intended line-drawing characters
instead of leaking DEC Special Graphics source letters into embedded PTYs.

TERMINAL-DEC-GRAPHICS-001 adds bounded, chunk-stable G0/G1 designation and
SI/SO translation ahead of the typed terminal snapshot.
TERMINAL-DEC-GRAPHICS-RELEASE-001 is complete in v0.1.236. Live Romulus session
8 rendered Kernel menuconfig with Unicode corners, lines, and tees and no DEC
source-letter leakage; navigation and normal exit completed `do_menuconfig`.
The optimized binary is installed and the initialized daemon was restarted.
The full workspace suite remains deferred until requested.

## M84 — Menuconfig presentation and keyboard ownership

Goal: show the initial Kernel and U-Boot menuconfig screen without requiring a
keypress, make PTY versus Yoctui keyboard ownership explicit and reversible,
and prevent false detached-terminal success messages.

MENUCONFIG-INTERACTION-001 adds the quiet-period screen flush and a model-owned
`Ctrl+G` foreground toggle that leaves the exact daemon session running while
the operator visits Yoctui and later resumes it. DETACHED-TERMINAL-STARTUP-001
adds explicit emulator profiles and bounded early-exit verification.
MENUCONFIG-INTERACTION-RELEASE-001 is complete in v0.1.237. Focused automated
coverage passes, and live Romulus Kernel validation confirms automatic ncurses
rendering, keyboard ownership, `Ctrl+G` leave/resume, normal exit, and a real
GNOME Terminal detached launch. Kernel and U-Boot share the validated typed
PTY/input path. Romulus compatibility did not expose an authoritative U-Boot
menuconfig task for a second live launch even though BitBake lists
`do_menuconfig` for `u-boot-aspeed-sdk`. The optimized binary is installed and
the initialized daemon was restarted. The full workspace suite remains deferred
until requested.

## M85 — Menuconfig failure recovery

Goal: prevent OpenEmbedded's interactive failure acknowledgement from leaving a
failed Kernel or U-Boot menuconfig session running forever and blocking later
platform operations.

MENUCONFIG-FAILURE-ACK-001 adds a bounded menuconfig-only prompt detector and a
single daemon acknowledgement, with split-output and live failure coverage.
MENUCONFIG-FAILURE-ACK-RELEASE-001 is complete in v0.1.238. Live Romulus
validation reproduced the U-Boot ncurses failure, retained its diagnostic
screen, automatically completed the acknowledgement wrapper, and then launched
the full Kernel menuconfig screen without a daemon restart.

## M86 — Device-tree decompile destinations

Goal: let operators choose where a DTB/DTBO is decompiled and immediately read
the resulting DTS without racing the compiler.

DTB-DECOMPILE-DIALOG-001 adds a typed destination form, bounded directory
browser, checked-by-default view option, and successful-PTY completion
correlation for Kernel and U-Boot. DTB-DECOMPILE-DIALOG-RELEASE-001 is complete
in v0.1.239; the optimized binary is installed and the initialized Romulus
daemon has been restarted.

## M87 — Daemon build-directory compatibility authority

Goal: make an explicit or recovered build directory initialize the daemon's
BitBake environment so Kernel, U-Boot, recipes, and configuration receive a
current capability snapshot after restart.

DAEMON-BUILD-DIR-COMPAT-001 carries the selected build environment through
daemon start/restart and reconstructs persisted workspace authority before
startup probes. DAEMON-BUILD-DIR-COMPAT-RELEASE-001 is complete in v0.1.240;
the installed daemon publishes current Romulus authority for BitBake variable,
recipe metadata, and recipe inventory operations.

## M88 — Reliable menuconfig interaction

Repair terminal key encoding, foreground toggling and client restart recovery
for Kernel and U-Boot, improve startup feedback, and publish v0.1.241 after
focused automated coverage and live menuconfig validation.

M88 input, recovery and preparation changes are released in v0.1.241. Live
Kernel input, Ctrl+G resume, and client restart pass. MENUCONFIG-UBOOT-LIVE-001
is also DONE after a build-local compatibility patch repaired the legacy
provider's C23-incompatible ncurses probe. Real U-Boot `mconf` session 24 passed
arrows, Enter, search, Ctrl+G resume, and same-session client restart recovery,
then exited without saving. Full workspace testing stays deferred.

## M89 — Recover transient BitBake capability failures

Repair sticky unknown backend capabilities after a temporary startup failure;
verify live fresh-client Kernel inspection and release v0.1.242. Existing U-Boot
provider and M67 evidence blockers remain independent.

M89 is DONE in v0.1.242. Live recovery from the daemon's retained unknown
snapshot passed, and a fresh initialized daemon/client loaded the real Kernel
configuration and menuconfig launch dialog with positive API authority.

## M90 — Readable Hardware document viewer

Correct Hardware PDF and schematic rendering so zoom samples a bounded
high-resolution page instead of enlarging a 320×240 preview. Keep the Navigator
usable beside the document, omit the Inspector to preserve viewing space, and
release v0.1.243 after focused model, app, CLI, and TestBackend checks.

M90 is DONE in v0.1.243. The reported 71-page PDF renders from a bounded
high-resolution source at fit and zoom, and the live Navigator remains operable
beside the document.

## M91 — Native Hardware document presentation and viewer exit

Replace the still-blocky full-page character preview with bounded native SIXEL
output on terminals that advertise it, provide crisp extracted PDF text when
native graphics are unavailable, and make `Esc` return to the Hardware library
regardless of whether Navigator or Workspace currently owns focus. Release the
focused correction as v0.1.244 after model, app, UI, CLI, and live Hardware
checks. The full workspace suite remains deferred until requested.

M91 is DONE in v0.1.244. Enabled GNOME Terminal profiles and terminals that
advertise SIXEL receive bounded native document pixels; unsupported terminals
open PDFs as crisp extracted text. The viewer returns to its retained Hardware
library from either visible pane.

## M92 — Snapshot-independent platform inspection

DONE in v0.1.245. Kernel and U-Boot / BIOS inspection use a locally owned,
read-only metadata scope while the daemon compatibility snapshot is absent or
refreshing. Menuconfig, builds, cancellation, events, and server control retain
daemon authority. Installed U-Boot inspection remained active with Braille
progress during an initialized Romulus daemon refresh and showed no
missing-snapshot notice. Full workspace tests remain deferred until requested.

## M93 — Readable Hardware PDF fallback

Reject unusable private-use PDF text extraction, make the non-native page
preview fit width for readable terminal rendering, and provide an explicit
Escape/Backspace return to the persistent Hardware library. Release the focused
correction as v0.1.246 after model, app, UI, CLI, and live reported-document
checks. Full workspace tests remain deferred until requested.

M93 is DONE in v0.1.246. Unreadable extracted font codes no longer reach the
viewer, non-native PDF pages use a fit-width scrolling raster, and Escape or
Backspace returns to the retained Hardware library row. The reported document
passed live forced-fallback validation. Full workspace tests remain deferred
until requested.

## M94 — Page-first detailed PDF fallback

Open every PDF as a visual page and replace the low-detail unsupported-terminal
PDF projection with a colored 2×4-dot cell renderer. Audit all five PDFs and all
92 pages under `~/projects/smarc`, then release v0.1.247 after focused model,
UI, CLI, and live document checks. Full workspace tests remain deferred until
requested.

M94 is DONE in v0.1.247. Every PDF opens as a rendered page, readable extracted
text remains available through `v`, and terminals without native graphics use a
colored 2×4 Braille projection. Poppler converted all 92 pages, and all five
documents opened through the optimized forced-fallback viewer. Full workspace
tests remain deferred until requested.

## M95 — Readable PDF graphics

Remove unreadable PDF character projection and false GNOME capability inference.
Validate complete SIXEL detection and native pixels in XTerm VT340 using all
five smarc PDFs. Release v0.1.248 after focused checks.

M95 is DONE in v0.1.248. PDF character projection and false capability inference
are removed. Native pages use cell-aligned tiles to avoid per-image clipping.
Real XTerm screenshots cover all five PDFs, zoom and library return. The user's
Terminator VTE build lacks SIXEL; it receives clear supported-terminal guidance.
Full workspace tests remain deferred at the user's request.

## M96 — Automatic readable graphics terminal

Make ordinary interactive invocation open its native-PDF-capable XTerm itself
when the current terminal lacks graphics. Select a readable scalable font and
useful starting grid while preserving the initialized environment and daemon
attachment. Release v0.1.249 after focused CLI, Hardware UI, and live XTerm
checks. Full workspace tests remain deferred until requested.

M96 is DONE in v0.1.249. `yoctui attach` now performs its own guarded handoff
from a terminal without native graphics to a readable 140x40 XTerm VT340 client.
The initialized environment, daemon attachment, working directory, and exact
arguments are preserved without a shell. Full workspace tests remain deferred
at the user's request.

## M97 — PDF wheel page navigation

Route mouse-wheel input over a Hardware PDF to bounded previous/next-page loads
and preserve keyboard panning. Release v0.1.250 after focused app, UI, CLI, and
live multi-page PDF checks. Full workspace tests remain deferred until requested.

M97 is DONE in v0.1.250. Wheel input over a PDF now advances bounded pages and
routes the resulting load through the existing Hardware worker. A real XTerm
wheel replay changed both the header and native pixels from page 1 to page 2 of
the 71-page SMARC guide. Full workspace tests remain deferred at the user's
request.

## M98 — RootFS composition chart continuity

Keep the RootFS package pie chart and exact composition list visible together
on ordinary wide 40-row terminals. Compact the chart vertically when needed
without removing package-selection or filesystem evidence; retain list-only
fallbacks for layouts and preferences that cannot render the Braille chart.

M98 is DONE. Wide 40-row workspaces render the pie and exact list together;
compact wide layouts stack them for readable table columns, while roomier
layouts remain side by side. Selection, filesystem evidence, and accessible
list-only fallbacks are preserved.

The v0.1.251 release packages this correction with focused UI, formatting,
strict Clippy, version-policy, roadmap, push, and optimized-build gates.

M98 is released in v0.1.251. The focused responsive coverage and release gates
pass; the optimized binary is built and the two coherent commits are pushed to
`origin/master`.

## M99 — Initialized QEMU Console launch

Restore the complete `Q` launch path from QEMU / Wic and Images: consume the
daemon's initialized absolute `runqemu` identity, refresh capability after
artifact scans, route on-demand inspection, and create the existing daemon-owned
QEMU Console PTY. Release after focused tests and a live initialized tool probe.

M99 is DONE in v0.1.252. On-demand and post-scan capability inspection use the
initialized daemon tool identity, and an approved launch opens the embedded
QEMU Console. The Romulus tool probe passes; no guest boot is claimed without
a compatible deployed QEMU machine artifact.
