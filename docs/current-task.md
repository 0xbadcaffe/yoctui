# Current Task

**ID:** DEMO-DEVTOOL-STATUS-SAFETY-001
**Title:** Prevent status inspection from initializing or enabling a workspace
**Status:** IN_PROGRESS

Native v301 recipe metadata works, but automatic upstream devtool status created
workspace/conf/layer.conf and README and enabled that layer without confirmation.
The added bblayers line was removed only after reconstructing and verifying the
original f1dc6040 checksum; generated workspace files remain, no user data removed.
Implement a fail-closed backend preflight after existing capability authorization:
absent default workspace is genuinely empty without launching devtool; an existing
workspace must already be enabled in a safely understood configuration before
status can run. Unknown/complex/custom configuration must report unavailable,
not initialize or infer membership. Preserve executable failures, cancellation,
Git inspection, explicit confirmed modify and daemon authority. Tests must prove
no process/config mutation for absent/disabled/ambiguous workspace and normal
existing membership, custom paths and fake-process cancellation. Update spec,
architecture/status; bump and full baseline; install current daemon/client and
repeat native recipe inspection while original configuration hashes stay exact.

```bash
cargo test -p yoctui-bitbake --all-features devtool
cargo test -p yoctui --bin yoctui devtool
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-ui-spec.sh
./scripts/verify-roadmap.sh
```

## Paused selected metadata environment child

**ID:** DEMO-NATIVE-METADATA-ENV-001
**Title:** Initialize the selected native build environment for local metadata workers
**Status:** IN_PROGRESS

Function-key dependency DONE code7bc634df/v300 optimized49d23642. Real plain
client2412326 read-only history -> F4 Dashboard/F6 nine layers; ordinary q clean0,
controller/client absent,29 prior rows exact, originals unchanged, daemon2340904
healthy295/NRestarts0 with unchanged wire. Evidence native-terminal-function-keys-
v300.txt. Full2075Rust/67bridge/strictClippy/fmt/source/UI/version/29rasters PASS.

Current atomic native defect: actual recipe inspection's worker previously called
select_backend_with_timeout with no selected build environment. Plain attach
has no caller bb Python path and handshake rejects the daemon-selected API.
Reuse the exact source/build initialization already used by platform inspection
inside the existing asynchronous recipe metadata/dependency worker; keep its
daemon-authorized backend negotiation, single-worker/returned backend/lifecycle,
real failure reporting and cancellation. Do not mutate caller environment,
upstream inputs or daemon capability records, synthesize API availability or
silently fall back to ancestor/wrong build/legacy source. Inspect shared helper,
recipe_inspection_operation.rs/backend_startup.rs; add normal/missing/wrong-source/
wrong-build/fake child import coverage and genuine native recipe/source view.
Update relevant architecture/spec/status, bump, baseline, commit/push/install.
Parent all-screen/current QEMU/docs/CI/reboot/publication remain pending.

Implementation v301: worker captures typed source alongside session build,
reuses existing selected platform environment through private metadata alias,
then passes that map to the existing daemon-authorized backend selector. No
ambient environment mutation or capability bypass. Four new fake-child tests
cover exact bb import, missing bb failure, initializer failure/no fallback and
abort/reaping; all six existing platform environment guards retained. Full2079
Rust/zero failures/nine unchanged ignores31targets/67bridge/strict workspace
Clippy/fmt/source2902/UI/version/roadmap/29rasters PASS.34goldens only300->301
identity, all other cells/styles exact. Optimized source-bound install and real
plain native recipe/source inspection still pending; task remains IN_PROGRESS.
Storage:58 validated obsolete pre300 internal dev/release libraries811577076B
and23 completed300 temporary test executables433683864B cleared after owner/
link/live-use checks; current outputs/external dependency caches/original data
and presentation/user captures intact. Rebuildable compiler cache only.

```bash
cargo test -p yoctui --bin yoctui metadata
cargo test -p yoctui --bin yoctui platform_inspection
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
cargo fmt --all --check
./scripts/verify-ui-spec.sh
./scripts/verify-roadmap.sh
# Manual: optimized plain native attach -> actual recipe metadata/provider;
# selected source/build bb module works without inherited PYTHONPATH; wrong
# environment still fails; original flash/config/symbol hashes unchanged.
```

## Completed DEMO-TERMINAL-FUNCTION-KEYS-001

Native installed299 audit reproduces F4 swallowed with a read-only notice in
Terminal Sessions. The existing global catalog requires Dashboard. Fix only
the shared CLI direct function-key route for ordinary terminal viewers/ended/
stale sessions; preserve embedded menuconfig forwarding, dialogs, replayed
context, live terminal controls and other workspace text handling. Add external
typed dispatch/reducer tests across lifecycle/authority/modal cases; native
installed-source F4 proof, bump/commit/push, full baseline and optimized install.
Parent native audit paused; recipe inspection also exposes a separate selected
build bridge environment failure. That atomic child follows this correction.

Implementation v300 restores the existing direct function-key catalog only for
ordinary Terminal Sessions, including empty/read-only/history/stale replicas.
Embedded menuconfig, replayed context, palette, other workspace text handling
and F12 menu access over existing editor dialogs remain unchanged. Two external
dispatch/reducer tests cover all ten function keys across three replica states,
seven lifecycles and local/remote/absent writer authority plus empty history,
palette/replay/dialog routing. Initial viewer regression fails before correction.
Full Rust2075/zero failures/nine unchanged ignores31targets and67bridge PASS;
strict workspace Clippy and optimized installed native proof PASS. All34 golden files
change only299->300 identity;29 deterministic raster checks PASS.23 validated
obsolete completed299 test executables433647264B cleared from explicit temporary
cache paths after owner/link/live-use checks; original data/current outputs intact.

```bash
cargo test -p yoctui --bin yoctui function_keys
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-ui-spec.sh
./scripts/verify-roadmap.sh
# Manual: plain installed attach -> read-only Terminal Sessions -> F4 Dashboard.
```

## Paused parent: DEMO-INSTALL-LIVE-001

Terminal children DONE. Latest code3d0d2940/v299 installed optimized3e03cc59;
actual plain native shell29/native GDB17.1 read-only matching vmlinux load,
resize/split/narrow/history/clean exit PASS. Native client2398207/shell2398264/
GDB2398369 gone, original28 rows/input hashes intact, daemon2340904 healthy295/
NRestarts0 unchanged protocol. Evidence native-terminal-viewport-v299.txt.
This proves local managed GDB sizing, not a new live QEMU kernel stop.

Resume actual native screen-by-screen rehearsal with final installed release:
recipe/layer/source/devtool/devshell/menuconfig, artifacts/rootfs/files/systemd/
D-Bus/udev and honest optional absent SDK/testing/security/QA. Preserve hardware
documents/progress; no physical actions or clean image rebuild. Rehearse reviewed
Yoctui OpenBMC flash QEMU boot/matching kernel GDB/breakpoint/source/reconnect/
cleanup on current source, record actual ownership and original hashes. Final
daemon upgrade requires checking no owned active jobs/PTYS before restart; allow
real metadata warm-up. Durable service/tools/profile already prepared, actual
coordinated laptop reboot is later separate acceptance, never infer it from restart.
Finish independent native/docs/CI work before requesting reboot coordination.

```bash
cargo build --release --locked -p yoctui --bin yoctui
# Manual: exact optimized installed source/hash and native all-screen/devtool/
# devshell/artifact/rootfs audit; reviewed real QEMU/GDB/reconnect/owned cleanup.
./scripts/verify-roadmap.sh
```

## Completed terminal viewport sizing (v299)

Dependencies writer and pane binding DONE. Exact v298 code44512807/rasterfc6cb51b,
optimized installedd538bd24; real shells27/28 separate split output, click27
routes ordinary input only27, close27 pane preserves both processes/selects28.
Normal exits both0 preserve26 prior rows. Client2385897 normal q/confirmation
exit0/terminal restoration, owned actors gone, daemon2340904 healthy295/NRestarts0.
Evidence native-terminal-pane-binding-v298.txt. Reboot/all-screen/CI pending.

Correct ordinary visible-cell resize, not a new layout/shortcut/wire: shell/GDB
120x40 clips within smaller panes. Generalize menuconfig geometry/polling to the
actual focused PaneId, mirroring tabs/access/prefix/border/status/search/history.
Preserve in-place menuconfig, zoom/narrow/split geometry and copy/search modes;
current live-owned writer only, no remote/stale/exited/zero-sized hidden resize,
unchanged coalesced transport/deadlines/budgets. Relevant app workbench_geometry,
CLI polling, external geometry/TestBackend/fake socket/native shell/GDB tests.
Implementation v299 underway: ordinary current writer polling, focused PaneId
geometry with prefix/search/history chrome and no invented empty cells. Four
new pure geometry regressions and three TestBackend cell-count crosschecks,
plus existing fake socket normal geometry/coalescing/remote/stale/ended guards.
Focused app61/UI34/CLI36(1existing manual ignore) PASS before bump. All three
new pre-fix geometry assertions failed as expected. Full299 baseline2073Rust/
zero failures/nine unchanged ignores31targets/67bridge/strict workspaceClippy/
fmt/source2902/UI/version/roadmap/29rasters PASS; all34 golden changes only
encoded298->299 identity, remaining symbols/styles byte-exact. Optimized install
and native ordinary shell/GDB acceptance PASS; viewport DONE. Exact code3d0d2940/
installed3e03cc59, real native sizes98x32/96x15/48x32/62x42/126x42/78x7/98x31
match shell/GDB/UI; normal GDB/shell exit then client q clean0,28 prior rows and
original inputs preserved. Evidence native-terminal-viewport-v299.txt. Storage preflight
cleared200 validated obsolete temporary compiler outputs2883105299B:23prior298
test executables433286864B,21old release outputs476857616B,156pre299 dev
libraries1972960819B; owner/link/live-use checks, current299/installed298 binary/
original source/images/symbols/assets preserved. Rebuildable Cargo cache only.

## Completed terminal pane binding (v298)

Dependency DEMO-TERMINAL-EXIT-WRITER-001 DONE: source8a30ba9f/v297 optimized
installed3b25186b, actual native shell26/helper2363644 exits0, historical matching
writer/epoch retained, current client2363585 observes read-only history and
ordinary q/confirmation closes clean0/restores terminal without F4/manual detach.
Healthy daemon2340904 remains verified295/917a9788; no backend/wire change or
restart needed for that client-side lifecycle proof. Final daemon upgrade and
actual reboot remain separate later acceptance, not claimed here.

Atomic prerequisite split before viewport code: actual native history has26
sessions, but UI terminal_session_panes chooses first pane-count session indices
whenever split. Selected owned shell/GDB26 can disappear from rendered panes
while input/inspector still address it. Mouse mapping has the same ordinal
assumption. Correct existing session/pane context, not a new layout or feature:
typed client-local PaneId/session bindings scoped to daemon identity, preserved
across split/session changes/focus/close, pruned for removed sessions/replacement.
UI/mouse/selected input/inspector must agree; no history/process deletion or
new wire/shortcut. Pure/reducer/app/TestBackend regressions for high history
index, two actual panes/outputs, mouse/focus/close and replaced/removed state;
real native two owned shells/split/session switching/clean owned exit required.
Relevant model terminal_workbench/terminal_selection/selection reducer, shared
app mouse mapping, UI pane projection and CLI existing prefix split/close routes.

Implementation v298 complete: shared typed projection and daemon-scoped bindings,
snapshot reconciliation and existing prefix/mouse/reducer routes. Five model
regressions, four TestBackend render regressions, real protocol replica replacement
and high-history mouse mapping pass focused checks (model42+worker1/app57/UI31).
Pre-fix selected26 output regression failed as expected. Existing synthetic
split fixtures now explicitly align initial focus with their selected session;
their original behavior assertions remain unchanged. Full rerun2066Rust/zero
failures/nine unchanged ignores31targets/67bridge/full strictClippy/fmt/source2900/
UI/version/roadmap/29rasters PASS, all34 goldens only297->298 identity. The first
full run exposed another inconsistent CLI mouse fixture; its intended session2
setup was corrected without weakening assertions and the entire suite rerun.
Optimized install and real native two-shell acceptance PASS; task DONE, evidence
native-terminal-pane-binding-v298.txt. Original inputs/26 prior rows unchanged.
Storage preflight cleared173 validated obsolete temporary compiler outputs:
90old dev libraries996455182B,23prior297 test executables433223448B,60old release
libraries963335943B; owner/link/live-use checks, original data/current outputs
preserved. These caches are rebuildable, not deleted source/image/debug assets.

Initial ordinary autosize reproduction for current DEMO-TERMINAL-VIEWPORT-001:
actual native ordinary GDB/build shells retain120x40 while the visible pane is
narrower/shorter. Current geometry and polling restrict resize to menuconfig.
Correct existing visible-cell contract for ordinary live owned PTYs, selecting
the actual focused PaneId rather than treating global session index as pane
index. Mirror actual borders/tabs/prefix/history/search status allocations.
Preserve menuconfig/in-place/zoom/narrow/split geometry, remote/stale/non-running
lease safety, bounded coalesced resize and copy/search modes. Relevant app
mouse/workbench_geometry.rs, CLI interactive_runtime/polling.rs and external
geometry/TestBackend/fake socket/live shell/GDB tests. No new layout/shortcut/
wire or relaxed budget. Bump/commit/push/optimized install/live source proof.

```bash
cargo test -p yoctui-model terminal
cargo test -p yoctui-app terminal
cargo test -p yoctui-ui terminal
cargo test -p yoctui --bin yoctui terminal
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 scripts/check-library-layout.py
./scripts/verify-ui-spec.sh
./scripts/verify-roadmap.sh
# Manual: native shell/GDB dimensions match rendered cells at160x50 and resize/
# split; visible cursor/output preserved, no remote/exited writer resize.
```

## Completed terminal lifecycle/writer correction (v297)

Model authority requires current Running selected session plus matching owner.
All three UI role surfaces (access/pane/inspector) project dead/retained roles
honestly; each split pane uses its own lifecycle. Seven new pure/reducer/app/
TestBackend/fake socket regressions cover stale/remote/lifecycle/paste exit race/
Quit traps/history/no resize wire request. Both old predicate and independent
pane role regressions fail before correction. Full297 rerun2055Rust/zero failures/
nine unchanged ignores31targets/67bridge/fullstrictClippy/fmt/source2895/UI/
version/roadmap/29rasters PASS; all34 goldens only296->297 identity. Native plain
installed exact release acceptance PASS. Evidence native-terminal-exit-v297.txt.
Prior296 release interrupted130 before install after review; initial297 full
Rust/bridge ENOSPC attempts not counted passing, both fully rerun. Only127
validated obsolete temporary compiler outputs/2522469334B cleared, original
inputs/current297 outputs/user captures preserved. Not all-screen/CI/reboot/
publication proof. Source2964e0c8a14 CI release-quality/compatibility PASS but
strict hosted async must_use and moved IPC source gates fail, still queued.

## Completed native bridge import verification (v295)

Atomic child split from the installed native screen audit before implementation.
Implementation v295: bridge-child bootstrap and six subprocess regressions;
pre-fix positive vendor case fails, post-fix all67 bridge tests pass. BitBake299/
UI377/strict workspace Clippy/fmt/source2892/UI/version/pinned29rasters pass;
all34 golden changes are only294->295 identity. Actual source-only native bridge
with only bitbake/lib returns virtual/kernel63tasks/nine packages/three sources
and STAGING_KERNEL_BUILDDIR with real conf/bitbake.conf:494 provenance, exit0.
Source-only check is distinct from the later installed native Kernel/config/
clean client-exit acceptance, which now PASS source62716abc/optimized917a9788.
Full workspace2048/zero failures/
nine unchanged live/manual ignores31targets PASS. check-docs reaches the real
native Doctor but fails its old argv JSON transport at183406bytes (OS per-arg
limit); retain that distinct harness defect for an atomic correction before
final docs/CI. Initial wrong-profile docs attempt filled only the temporary
compiler cache; owned generated failure outputs cleaned, low-space rerun used.
Plain client workers initialize the selected source/build with only bitbake/lib.
Before correction, the actual retained BitBake2.19.1 generated parser imported
pysh.pyshtables and failed ModuleNotFoundError: pysh. The daemon's explicit
lib/bb path worked; local Kernel inspection fell back to an unavailable legacy
adapter. Reproduction is recorded in native-rehearsal-v294.txt.
Correction initializes only the imported real bb package's existing vendored package parent
inside the owned Python bridge process, not the laptop/client environment or
upstream source. Preserve old single-module adapters and genuine import failures;
never synthesize API availability, change selected implementations or relax bounds.
Relevant bridge tinfoil_workspace.py and external bridge import regression tests.
Package/legacy/no-vendor/failure subprocess coverage, embedded bridge checks,
version bump, installed native Kernel/config inspection and source evidence
passed. No UI layout/wire change. Native parent resumes after terminal children.

```bash
python3 -m pytest bridge/tests
cargo test -p yoctui-bitbake --all-features
cargo fmt --all --check
cargo clippy -p yoctui-bitbake --all-targets --all-features -- -D warnings
python3 scripts/check-library-layout.py
./scripts/verify-roadmap.sh
# Manual: installed current source plain native attach -> Kernel inspection;
# no inherited PYTHONPATH workaround, original artifacts/config unchanged.
```

## Parent native rehearsal (in progress)

Warm v294 rootfs retry actually resolves IMAGE_ROOTFS:2628entries/195offline
systemd units, real root(0) owners/modes/size; systemd End reaches195/195. The
filesystem package-ownership mapping remains explicitly unavailable, not image
uid/gid. Driver uses global F4 before q/y; native controller93428 closes clean0.
Early cold timeout remains a recovery caveat, not silently bypassed/relaxed.

Full local quality DONE product399ce139/v294 (handoff02842aeb):2048Rust/61bridge/
strictworkspaceClippy/fmt/fullrelease/docs/source2891/roadmap PASS;9 existing
ignores unchanged. Reconciled proof order: native cold setup -> optimized real
screen/boot/GDB rehearsal -> final native README/screenshots/flamegraph/runbook
-> hosted CI/final source install -> coordinated actual post-reboot proof.
Publication waits for CI and native parent, including real post-reboot check.
Native parent split into this cold-setup child and DEMO-NATIVE-POSTREBOOT-001.
Do not label cold restart as actual reboot, or stop independent CI work merely
because a laptop reboot needs coordination. Never reboot uncoordinated.

Dependency native bootstrap DONE: installed ~/.cargo/bin/yoctui v0.1.294,
built7eef0058 SHA c0f800a1462fcc2acfc1c2347a997821ccb870b7014f181a3ab9c7254ad22c72,
optimized thinLTO/debug1/frame-pointers/two Cargo workers. Native Ubuntu26.04/
Python3.14/BitBake2.19.1 real metadata/API4799recipes/nine layers/Romulus/two
workers PASS. Durable native QEMU11.0.2/GDB-multiarch17.1 login-PATH wrappers,
exact initialized source/sibling-build profile and enabled user service work.
Normal /run/user/1000/yoctui endpoint; plain native attach/cold restart PASS.
Current PID2315820/instance2745eabac4955de353e2af293d5c3541, metadata ready.
First native scan349s/cached restart222s; do not confuse warm-up with failure.
117available/16unknown/5unavailable capabilities are not universal live proof.
Evidence artifacts/live-openbmc/romulus/native-bootstrap-v294.txt. Same boot ID:
actual reboot NOT verified. Owned idle container daemon stopped after actual
snapshot/process/client checks; unrelated ZCU daemon/clients remain untouched.

Use the installed native release and existing real source/build/profile for
independent all-screen audit: actual recipe/layer/config/devtool/devshell,
rootfs/packages/services/artifacts/file views, honest missing SDK/testing/etc.
Review real QEMU boot/matching kernel GDB start_kernel breakpoint/source,
continue to OpenBMC login, detach/new client/reconnect/interrupt/backtrace/
registers and owned helper/guest/GDB/socket/staged-copy cleanup through Yoctui.
Never use unmanaged QEMU or container-only screenshots as native acceptance.
Preserve original image/debug/config hashes and user history/hardware documents;
no full image rebuild, root/policy changes, deployment or physical action.
Native capture driver and assets are above project in presentation directory,
remain uncommitted/unpushed. Click actual visible truncated Navigator labels
when the Layers navigator is narrow, not guessed full text. Raw PTY capture
answers CPR from actual composed cursor; client close must restore terminal/
exit0. Record exact paths/source/hash and unavailable optional capabilities.
Prepare reproducible two-minute path/recovery checklist for final docs task.
Split only genuinely observed correctness defects before implementation; no
new features. Hosted CI atomic/version-policy/moved-source checks follow docs.
Actual coordinated post-login reboot is a separate later child, no automatic
reboot/session replay. Publication remains gated by that actual proof and CI.

```bash
cargo build --release --locked -p yoctui --bin yoctui
# Manual: current installed optimized source-bound release hash/version
# Manual: real native screen-by-screen/devtool/devshell/artifact/rootfs audit
# Manual: actual reviewed Yoctui QEMU boot/GDB/reconnect/owned cleanup
# Manual: original inputs/unrelated work preserved; optional limits explicit
./scripts/verify-roadmap.sh
```

Completed local polling fix: source040a23d0/v270 exact optimized49f75463 release,
same idle Layers/unchanged daemon10s warm60samples: client15.276970->0.270380%
oneCPU (98.23% reduction); combined16.256828->1.354049%. Actual after188samples/
zero lost/App::clone0.89% inclusive with legitimate guarded copies preserved.
Evidence artifacts/performance/demo-v270, flags/RSS/descendants documented.
Not full-suite/M46/M67 certification or installed/published final release.

## Completed profiling baseline / release polish scope

DEMO-PROFILE-001 DONE: fresh valid source8761f308 fixture flamegraph2299samples/
6000frames/4580ms and current workbench checksum; real idle/startup profiles and
exact CPU record, full-suite baseline88failures/6targets and bridge61PASS.
Evidence artifacts/performance/demo-v269. Atomic children: background copies,
selected source initialization, contract fixtures, verified UI clipping and
integration failures, then full verify/docs/install/rehearse/publish.

New user instruction: no new features; polish measured hotspots and correctness,
update README screenshots/latest measured report/operator guide, bump version,
run full suite, install optimized release, verify real OpenBMC daemon/attach/
every demo screen and publish the verified public crate graph to crates.io.
Full-suite deferral is explicitly revoked. Dependency OPENBMC-QEMU-GDB-LIVE-001
is DONE. Additional workstation acceptance: plain native `yoctui attach` and
reliable demo startup after laptop reboot/login, not a container launcher.
Durable tools, exact saved OpenBMC profile and enabled user service must replace
temporary setup; runtime endpoints recreate normally. Live processes do not
survive reboot. Cold-restart checks and an actual coordinated post-reboot check
are distinct; never reboot this working laptop without user coordination.
Dependency OPENBMC-QEMU-GDB-LIVE-001
is DONE. Start with source-bound optimized fixture flamegraph and real retained
OpenBMC client/daemon profiling; preserve prior evidence and record workload,
sampling, source/binary identity, timing and permission/storage prerequisites.
Relevant scripts/flamegraph.sh, CLI workbench_profile bench and real capture
scripts/evidence. Run full-suite baseline to identify existing failures, then
split actionable hotspot/correctness fixes into atomic children before code.
No new layout/workflow, long clean image rebuild, physical action or ZCU102 retry.
Use two compile workers/existing temporary target due limited storage; do not
delete sources/images/debug symbols. Registry/publication credentials must not
be printed. Presentation and its screenshots stay outside Git until explicitly
selected copies/provenance for README; presentation itself uncommitted/unpushed.

Verification:
```bash
# Manual: real source-bound perf samples before and after selected fixes;
# distinguish fixture stress from actual OpenBMC client/daemon runtime.
cargo test --workspace --all-features --no-fail-fast
./scripts/test-flamegraph.sh
./scripts/verify-roadmap.sh
```
Done requires committed baseline/findings with exact evidence and eligible
child tasks; immediately continue polish/full verification/docs/install/demo/
publish. Do not mark unavailable optional integrations or old external gates
complete from mock/screenshots. M67 current-Poky/physical KGDB/instrumentation
remain independent blocked prerequisites; new OpenBMC profile is not M67 proof.

## Previous external instrumentation handoff (not active)

Dependency KERNEL-INSTRUMENTATION-UI-001 is DONE. This is the highest-priority
required incomplete task; no independent eligible implementation remains.
External dependency: an approved separate instrumented build/boot/reproduction
scope, provider/version/architecture/compiler and matching resolved .config/
runtime diagnostic evidence. The retained OpenBMC kernel is not an approved
instrumented runtime. Do not enable destructive tests, alter its config/layers,
deploy/reset hardware or fabricate diagnostics to bypass this dependency.
Relevant files: kernel instrumentation model/app/UI/CLI and retained live logs.
Done requires each supported preset retained in matching resolved config and
genuine controlled authorized runtime diagnostics; configuration export alone
is not runtime compatibility. Record exact provenance, update registry/status/
roadmap and commit only after verification. Full suite remains deferred.

Verification when external prerequisites are supplied:
```bash
# Manual: approved separate build, provider/version/architecture/compiler,
# reviewed fragment integration and resolved .config; deliberate build/boot,
# controlled authorized reproduction and matching logs separately per preset.
./scripts/verify-roadmap.sh
```
KGDB-SERIAL-LIVE-001 also needs an approved already-configured board and exact
matching running-kernel symbols/config with exclusive host serial transport.
M67-LIVE-EVIDENCE-001 needs genuine current-source real-Poky performance evidence.
Neither is certified by QEMU boot/debug. Deferred ZCU102 tasks are not retried.

## Completed OpenBMC/QEMU handoff — M113

OPENBMC-QEMU-GDB-FLASH-001 and OPENBMC-QEMU-GDB-LIVE-001 are DONE.
Product v0.1.269 committed/pushed as361ff4484104; exact optimized two-worker
source-bound release target/release/yoctui built and hash-verified:
785779c2502b452897efd41c6aabbbc0ccd94946f1dcd0d133050d80a6e9081b.
Real independent daemon-owned reviewed flash launch, U-Boot/Linux/OpenBMC login,
matching ARM DWARF start_kernel breakpoint/source, resume, fresh-client reconnect/
interrupt/backtrace/registers and owned QEMU/GDB/socket/staged-copy cleanup PASS.
Original flash/config and selected matching package artifacts unchanged.
Exact failures/prerequisites/recovery/limits and identities are recorded in
artifacts/live-openbmc/romulus/managed-flash-debug-v269.txt. No image build/deploy.
Four genuine screenshots/raw PTY cells and full serial log saved under
/home/bspguy-dev/projects/yoctui-ydd2026-presentation/assets. Updated editable
PPTX/ODP slides17-19; LibreOffice/24slides/25minutes/notes/editability/layout and
reference checks PASS. Presentation remains outside Git, uncommitted/unpushed.
Focused tests/strict workspace Clippy/fmt/UI/version/roadmap checks passed;
no full suite. ZCU102 source/build/debug work preserved and deferred by user.

## Historical ZCU102 handoff — deferred by user, not current work

**Resume hint:** `ZCU102-RESUME`
**Resumed:** 2026-10-02 by the user's request to finish kernel debugging.
The earlier pause is revoked. BUILD remains externally storage-blocked.

Latest read-only authority (2026-10-02 16:27 UTC, IPC sequence32861):
job3 Failed/exit1, aggregate3524/10994, completed success=false at
1790948119425ms. Native HALT at909840384bytes stopped the remaining work;
the private daemon0b20c2dd7719990023902d8d24d1ada8 is still alive and idle.
No duplicate build or daemon restart was requested. Available disk is now
6018834432bytes (~5.6GiB), too little headroom for a reliable image retry.

New retained partial output: linux-xlnx's vmlinux.unstripped has .debug_info
and .symtab, SHA256b696c59669a570410c97e7e9f5ff85326d017e9eb9972247eb864ea619df75c9,
build ID5f76474462b0eb96d23bb62f5c78c1ab2c83060a. Config hash remains
2fed27779d654cb5afde0893e218a7892baa541e49f77425815e760f46b38e9a.
Final vmlinux/arch/arm64/boot/Image and deployed rootfs/qemuboot are missing;
this linked ELF is not a completed image or matching boot/debug acceptance.
Keep it and all current kernel work. Exact inspection is in
artifacts/live-xilinx/zcu102/resumed-kernel-debugging.txt.

Additional cleanup is awaiting the user's choice. Initial old native compiler
work estimate9GiB was corrected: only3.2GiB of Clang/LLVM build directories
was identified, including binaries that must be preserved; actual removable
intermediates are less. Rust's4.1GiB is source, not disposable build output.
No new cleanup is authorized/performed and prior approved cleanup is exhausted.
No alternate storage mount exists. Obtain enough safe space before a fresh
explicit daemon retry; preserve sources/Git/images/debug/installed tools and
unchanged4GiB stop/1GiB halt/two-worker limits. GNOME remains inactive and
runtime-masked, with private tag backup/restore instructions preserved.
Physical KGDB and instrumented-kernel live tasks remain separately blocked;
SysRq/kdump remains proposed, not silently completed by existing crash analysis.
No eligible independent registry implementation task remains. Full suite deferred.

Historical paused checkpoint (superseded by terminal outcome above):

Latest authority: job3 hit the unchanged native4GiB STOPTASKS guard at
1790947062183ms (13:17:42UTC), free3.999GiB. New tasks stopped; kernel
linux-xlnx:do_compile PID1781068 is still draining. Read-only IPC13:22:24UTC,
sequence31591: job3Running/exitnull, aggregate3524/10994 (job checkpoint3513).
~3.8GiB free. This is incomplete/blocked, not a terminal failure yet and not
image success. Leave the active daemon/kernel/container untouched. U-Boot fetch
succeeded; compiled native QEMU/Vim and current kernel work are retained.
No deployed regular image files or matching vmlinux verified; QEMU/GDB not begun.
Current compile .config SHA2562fed27779d654cb5afde0893e218a7892baa541e49f77425815e760f46b38e9a;
DWARF5/KALLSYMS enabled, randomization disabled, optional GDB scripts disabled.
The earlier precompile config hash is historical, not final ELF matching proof.

For any further retry, read actual daemon/storage authority FIRST, not submit a build.
If job3 still runs, preserve/drain it. If laptop/container stopped, start only
the retained validation container/private daemon and check recovered outcome;
persisted Running is not proof of a live task. A fresh explicit image retry
needs additional safe storage beyond already completed approved cleanup and an
idle daemon; do not weaken guards or remove source/Git/images/debug files.
All other incomplete registry tasks are externally blocked or depend on BUILD.
Desktop index is still temporarily paused; private tag backup/restore details
are in storage-cleanup.txt. A reboot clears the service's runtime mask: reassess
indexer/storage before restarting a build. No full suite.

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
checks; both releases retained/hash-verified. ~8.0GiB was free before retry. Exact cleanup/
restore evidence is storage-cleanup.txt. GNOME desktop index remains temporarily
runtime-masked until storage permits restoration; private tag backup retained.

Historical running checkpoint before latest disk guard above: job3 RUNNING (accepted12:29UTC, native
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
./scripts/live-zcu102.sh status
python3 scripts/inspect-zcu102-build.py | jq '{sequence,build_progress,jobs,last_build_events:[.last_build_events[]|del(.data)]}'
./scripts/live-zcu102.sh attach
./scripts/verify-roadmap.sh
# Only AFTER additional storage and an idle/ready daemon:
./scripts/live-zcu102.sh build
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
