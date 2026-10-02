# Current Task

**ID:** KGDB-SERIAL-LIVE-001
**Title:** Verify KGDB attach backtrace resume and re-entry on a real board
**Status:** BLOCKED

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
No other eligible implementation task remains. Sanitizers/lockdep and SysRq/
kdump remain proposals requiring independently scoped tasks. Full suite deferred.

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
