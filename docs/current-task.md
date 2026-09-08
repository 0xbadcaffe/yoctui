# Current Task

**ID:** RELEASE-PERF-REFRESH-001
**Title:** Refresh source-bound real-Poky performance evidence after OpenBMC integration
**Status:** DONE

All 705 registry tasks are DONE. The bounded telemetry correction and its
source-evidence parent are verified together by the actual fresh capture.
The complete ./scripts/verify-performance.sh --real-poky-evidence chain passed;
session 8793 exited 0, log /tmp/yoctui-v89-performance-real-gate.log. This is
the terminal task handoff, not a full completion claim. Finish final docs/
roadmap/version checks, commit the coherent v0.1.89 change, then continue
immediately to ./scripts/verify-completion.sh in an exact clean worktree as
needed below. No required task implementation remains; the global gate must
still pass before stopping. If it exposes a new defect, register it explicitly.

## Verified correction and measurements

v0.1.88's actual 360-sample real compile failed combined CPU at 1.0334662486%.
Its separately labeled diagnostic profile contains 275 samples, zero lost
samples and zero unresolved weight: full snapshot serialization is 26.90%
inclusive cycle weight. The profile conversion exceeded the controller timeout
after recording, completed later, and passed the unchanged summarizer separately.
All diagnostic processes exited. Failed and profiled records remain distinct
under artifacts/performance/real-poky/v88-* and profiles/v88-*.

The v89 correction adds only Telemetry to the existing conservative snapshot
size-ledger path. It changes no timers, thresholds, event payloads or workload.
Two failed-first tests reproduced 101 full serializations for 100 telemetry
events and absent headroom amortization. Four regressions now pass: unchanged
snapshot/ordered replay, exact remeasurement and bounds, atomic size rejection,
and sequence/generation exhaustion. Actual source patch is retained at
artifacts/performance/real-poky/v89-source.patch against d2214e8; reapply with
git apply --unidiff-zero. No runtime source changed after the measured build.

Fresh unprofiled idle results pass unchanged limits: daemon 0.0416005%, client
0.1247861%, combined 0.2703778% of one logical CPU. There are ten-second warmups
and sixty samples per daemon/attached scenario, private XDG and no build env.
Exact output under artifacts/performance/results/low-overhead-v89 is copied
byte-for-byte to the canonical low-overhead directory after validation.

Fresh unprofiled real compile: 360 samples after ten-second warmup, combined
CPU 0.5831064335%, daemon 0.1788248486%, client 0.3959064743%. Host utilization
99.7892%, BitBake tree 308.6674% of one logical CPU. Input p95 4.558337 ms;
build/cancel acknowledgement 0.676482/6.060129 ms, fresh attach 50.053626 ms.
Queue maximum 90/256, 1.289 fps, no drops, reliable waits, resynchronization
or backend disconnect. Reconnect/cancellation passed and all owned processes
exited. Session 47552 completed with exit 0; /tmp/yoctui-v89-real-poky.log.
The unchanged full real-Poky Python validator passes exact source/binary
provenance and all metrics. No Cargo or profiler ran during either capture.

Canonical real-Poky record/manifest now match the exact retained
v89-linux-yocto-do-compile.json, all runtime Rust/bundled bridge hashes and
v89 source patch. v64 records remain in Git history, failed v88 evidence remains
unchanged. The regenerated regression record/manifest passes 22 hard metrics
and seven correctness checks; builder regressions pass. See docs/performance.md.

## Candidates and baseline

- Measured release: /home/bspguy-dev/.local/state/yoctui-v89-release.N94Frh/yoctui
- SHA-256 261fb7a45026d7a869c9ae0f8804b7c813c226d2e3d62238fad76b0c795908fd
- Release build passed in 34m 44s, /tmp/yoctui-v89-release-build.log; session 69078 closed.
- Workspace-tested debug: /home/bspguy-dev/.local/state/yoctui-v89-validated.mVei4c/yoctui
- SHA-256 f93458734ee142a4f0b5535db003264ff2327f619466918b3422938a3c620e4e

All 1599 workspace tests/doc-tests (four existing ignored), 52 bridge tests,
strict Clippy, fmt, docs, version policy and 705-task roadmap pass. Seven
readiness/version/regression script tests and three idle-harness tests pass.
All 17 golden diffs are version digits only; six rasters regenerated/verified.
Debug candidate was preserved before docs rebuilt the default-feature binary.

Required baseline: cargo fmt --all --check; cargo test --workspace --all-features;
cargo clippy --workspace --all-targets --all-features -- -D warnings;
python3 -m pytest bridge/tests; ./scripts/check-docs.sh; ./scripts/verify-roadmap.sh.
Every commit bumps patch version, 17 goldens and six rasters. All Cargo commands,
including docs, run sequentially with CARGO_BUILD_JOBS=1,
CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0,
RUST_TEST_THREADS=2. Never run Cargo or profilers during acceptance measurement.

The global gate requires a clean tracked checkout; its version checker also
treats the four unrelated user-owned untracked captures as a pending commit.
Preserve them: use an exact clean Git worktree for the final gate if needed,
not removal/staging/hiding or weakened checks. This may rebuild artifacts due
to source paths. Tools and stable/nightly toolchains are installed.
If enabling final live smoke, set YOCTUI_LIVE_BUILD_DIR to the isolated fixture
below and YOCTUI_OE_INIT_BUILD_ENV=/home/bspguy-dev/src/poky/oe-init-build-env.
Review the smoke's owned target/cancellation behavior before running it.

## Exact private performance fixture

- Build /home/bspguy-dev/.local/state/yoctui-release-poky.xWaZbs
- Source /home/bspguy-dev/src/poky, supported 6.0.2; qemux86-64/poky,
  BitBake 2.18.0, linux-yocto 6.18.24+git
- Private TMPDIR/DL_DIR/SSTATE_DIR; read-only source/sstate mirrors and shared
  Git object pools; offline, eight build/make workers, STOPTASKS 15 GiB / HALT 8 GiB
- Read-only bitbake -e verified every writable work/cleanup path inside this fixture.
- local.conf SHA-256 009dc5ebdd43ce7f207d92a7ec24f8d88241abc88086d7c6e07f2ac3205f2e5f
- bblayers.conf SHA-256 38e5a5753846f853f8adfb9db768b19cecbd36bce32058e39e7fb2f7ff3e3496
- v89 preflight: exact hashes, no Cargo/BitBake jobs, 26 GiB disk/11 GiB RAM available.

Only this newly created fixture may receive daemon-owned kernel cleansstate/
compile. Never clean original Poky builds or old performance fixtures.
Recheck exact identities and resource headroom before lifecycle actions.
All original Poky/OpenBMC data and pre-existing source edits remain preserved.

## Preserved live authority

- Source /home/bspguy-dev/src/openbmc, clean d4fd7d3f54e88e800c0284b753af68a13aabbef6
- Build /home/bspguy-dev/src/build-openbmc-romulus; MACHINE romulus;
  DISTRO openbmc-openpower; BitBake 2.19.0
- Private daemon PID 2355755, instance 2b9f400ab1ea2f7b79370a6831705b43;
  re-read daemon.json and executable hash before any lifecycle action
- Binary /home/bspguy-dev/.local/state/yoctui-v86-command-validated.mdKFIT/yoctui
- SHA-256 d24d35b2ea13af0bf28e82556b540148e80643dafea3a7b4e0aa3b4ce0a86f4b
- Private XDG config/state under
  /home/bspguy-dev/.local/state/yoctui-openbmc-validation;
  runtime /run/user/1000/yoctui-openbmc-validation
- Old image daemon v0.1.76 stopped normally after success. v0.1.84 recovered
  jobs 1/2 unchanged; real llvm-native/stdplus listtasks used job 3, exited 0,
  2/2. Queue/start/completed PNs and observed compacted timestamps matched;
  two fresh attachments froze measured 1814 ms at 00:00:01 with no ghosts.
- Private v86 now retains jobs 1/2/3 unchanged. Its first 35bf candidate
  rejected a real command/API mismatch and stopped normally while idle;
  the current d24d candidate corrects it. Keep those evidence identities distinct.

Evidence is in artifacts/live-openbmc/romulus. The generated 32 MiB MTD and
XZ SquashFS match the retained rootfs. One upstream render-group sysusers
warning was traced, not patched/suppressed; runtime impact is untested.
No firmware boot or completion-gate success is claimed. Normal installed
release remains v0.1.64. Preserve all Poky data and all four user-owned untracked
retry-active-dashboard captures. No image rebuild is needed.
