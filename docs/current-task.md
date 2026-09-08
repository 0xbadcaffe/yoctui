# Current Task

**ID:** RELEASE-CI-093
**Title:** Prepare v0.1.93 README and repair hosted release CI
**Status:** DONE

User override: update the front README version, repair GitHub CI, commit/push
and publish the current build to crates.io. CI run 34247832320 identifies
shallow checkout ancestry failures, missing pinned raster dependencies, and
PTY startup/exit synchronization. Relevant files: README.md,
.github/workflows/ci.yml, scripts/check-ci.sh, scripts/test-tui-{pty,keymap}.sh,
version/golden/raster artifacts and release documentation. The CI contract,
both real PTY gates, docs, README/version/roadmap checks and package graph
verification passed, as did 1600 workspace tests/doc-tests (four existing
ignored), strict Clippy, fmt and 52 bridge tests with 78.35% coverage. An actual
Ubuntu 26.04 container reproduced all sixteen PNGs exactly. All 710 registry
tasks are DONE. Delivery now requires pushing this verified candidate,
observing hosted CI and publishing the six public crates. Keep the four
user-owned OpenBMC retry captures untracked.

The v0.1.92 clean-worktree completion run was stopped during compilation of
the optimized UI benchmark to prioritize this request. It has not passed.

## Previous completed task and retained evidence

All 709 registry tasks are DONE. README-SCREENSHOTS-001 is verified in v0.1.91
and pushed at 4b60d22 before the recoverable fixture-readiness work was
restored. The four unrelated untracked OpenBMC retry captures remain user-owned
and untouched. This parent is complete after its two atomic follow-ups:
RELEASE-IPC-SOURCE-001 (v0.1.90) repaired the obsolete fail-closed source
assertion; RELEASE-FIXTURE-READY-001 (v0.1.92) completes source-bound actual
fixture readiness and evidence validation. The remaining repository-wide
completion confirmation must run in an exact clean worktree.

The event-flood fixture previously sent `start_build` while the initial recipe
inventory was loading, ignored the correct typed conflict, then misreported a
missing generator result. The bounded repair waits read-only for authoritative
metadata: exact daemon identity, workspace/build directory, sequence and
generation, workspace event, and ready log. It fails closed on timeout, EOF,
metadata failure, order/identity violations and immediate command rejection.
There are no blind sleeps or retries of accepted commands. All ten harness
tests pass, as do actual `--backpressure`, performance-CI-fast and
bounded-memory validations.

The preserved v89 release binary passes the fresh actual flood after 0.100 s
of readiness synchronization: 4002 ordinary events, maximum queue 603,
slow-client disconnect/reconnect, no forced resync and retained critical
events. The fresh required 30-minute observation has daemon/client RSS growth
2330624/282624 bytes, final-window slopes 6397.72/1781.54 B/min and stable
three/one threads; critical retention, order, continuity and owned-process
cleanup pass. Historical records remain preserved separately. No daemon runtime
source, threshold, workload or timer changed, and no Cargo or profiler ran
during acceptance.

Canonical IPC, memory and regression manifests bind the actual fresh
source-bound evidence; historical event-flood and memory records/manifests are
preserved under their `pre-readiness` names.
The exact harness/test source patch against db7a6a7 remains
artifacts/performance/ipc-gate/v91-readiness-source.patch, SHA-256
5af884b2628c26ccc9e685a922f9e70e4737726b5c7e4eca07ddff3f8b1a759f;
reverse check with git apply --unidiff-zero passed.

Use the preserved v89 release 261fb7a4 below for these actual fixture captures:
the checker/harness changes leave all runtime source hashes unchanged. Do not
claim a v91 binary was measured. Harness SHA-256 at final flood capture:
3072d5ae82e96bf07dc597debebf29de674df3c07e52c619c5306c1e71426b42;
measure-bounded-memory.py is unchanged (63238e10). No Cargo/profiler runs
during acceptance. Logs /tmp/yoctui-v91-{readiness-same-binary,release-flood,
memory-preflight}.log; failed-first readiness log
/tmp/yoctui-v91-readiness-failed-first.log. Rebase version/governance artifacts
to v0.1.92 without changing measured source or relabeling the v91 observations.
Canonical memory/regression promotion and the full baseline passed in v0.1.92.

Required checks are complete: ten harness tests, actual IPC backpressure,
performance CI fast, retained/dynamic bounded memory, workspace tests, strict
Clippy, bridge tests, formatting, docs, roadmap, version and render checks.
The next and final operation is `./scripts/verify-completion.sh` in an exact
clean worktree.

## Preserved release background

The v89 telemetry repair and source-evidence parent are committed at 11f3d8f.
Their complete ./scripts/verify-performance.sh --real-poky-evidence chain
passed; session 8793 exited 0, log /tmp/yoctui-v89-performance-real-gate.log.
Failed checker/fixture diagnostics and the exact preserved diagnostic binary
are documented in artifacts/performance/ci/v89-ipc-gate-failures.md.

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
