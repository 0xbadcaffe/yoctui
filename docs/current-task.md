# Current Task

**ID:** RELEASE-DAEMON-CPU-001
**Title:** Profile and correct real-build runtime CPU above the release budget
**Status:** IN_PROGRESS

Dependency OPENBMC-LIVE-001 is DONE. Parent RELEASE-PERF-REFRESH-001 waits
for this separately registered runtime investigation. Commit this v0.1.88
split, harness repair and failed evidence before runtime implementation.
All baseline checks now pass. No runtime hotspot or correction is established yet.

## Observed failure and acceptance

The actual v88 linux-yocto do_compile capture completed 360 samples after
ten seconds of warmup. Combined CPU is 1.0334662486% > the unchanged 1.00%
one-logical-CPU ceiling. Daemon/client independently trimmed means are
0.5872427701%/0.4039586047%; never add those to claim a combined pass.
The combined metric trims each sample's daemon-plus-client sum. Latency,
saturation, continuity, reconnect and owned-job cancellation passed.
Preserve artifacts/performance/real-poky/v88-linux-yocto-do-compile.json,
v88-failed-manifest.json and v88-source.patch. Canonical historical v64
evidence remains unchanged and fails current source hashes. The old tmpfs
fixture differs from the new ext4 fixture; no code regression is established
by these two observations alone. See docs/performance.md for exact metrics.

Profile the preserved v88 release binary under a separately labeled real
workload. Identify a measured hotspot, add focused normal/failure/safety
regressions, then make one bounded correction. Preserve event correctness,
input/cancellation responsiveness, bounded work and backend lifecycle.
Profiles never satisfy unprofiled CPU acceptance. DONE requires focused tests,
full baseline and a fresh exact-release source-bound real-Poky capture passing
all unchanged gates. No Cargo or profiler during acceptance measurement.

The v88 harness now waits read-only for asynchronous initial workspace
metadata before any commands or measurement. Three failed-first regressions
cover heartbeat, replacement authority, metadata failures, EOF and timeout;
an actual guarded no-command probe reaches its intentional READ_ONLY_STOP.
This is functional evidence only. Startup probe evidence is preserved in
artifacts/performance/real-poky/v88-readonly-startup-probe.txt.

## Exact private performance fixture

- Build: /home/bspguy-dev/.local/state/yoctui-release-poky.xWaZbs
- Poky: /home/bspguy-dev/src/poky, supported 6.0.2; qemux86-64/poky,
  BitBake 2.18.0, linux-yocto 6.18.24+git
- Private TMPDIR/DL_DIR/SSTATE_DIR; read-only source/sstate mirrors;
  offline mode, eight build/make workers, STOPTASKS 15 GiB / HALT 8 GiB
- Read-only bitbake -e confirms every writable work/cleanup path is inside
  this exact fixture. Only mirror URLs and shared Git object pools read old caches.
- local.conf SHA-256
  009dc5ebdd43ce7f207d92a7ec24f8d88241abc88086d7c6e07f2ac3205f2e5f
- bblayers.conf SHA-256
  38e5a5753846f853f8adfb9db768b19cecbd36bce32058e39e7fb2f7ff3e3496
- Exact v88 release: /home/bspguy-dev/.local/state/yoctui-v88-release.TBqp2N/yoctui
- Binary SHA-256
  be5fa7261cd3ebaf7f6f1786eabb46736419d043086d0e92224f3cc63f48b510
- Release build passed (35m 26s), /tmp/yoctui-v88-release-build.log.
  Unprofiled capture completed and cleaned up, /tmp/yoctui-v88-real-poky.log.

Only this newly created fixture may receive daemon-owned kernel
cleansstate/compile. Never clean the original Poky build or old performance
fixture. Recheck disk/memory, no competing Cargo, exact process identity and
job ownership before each run or manual lifecycle action. Normally let the
capture own cleanup. Preserve all original Poky/OpenBMC data and source edits.

## Verification

v88 release build, 52 bridge tests, three readiness tests, four version/
regression-record tests, six raster tests and version policy pass. All 17
golden edits were mechanically checked as version digits only. All 1595 v88
workspace tests/doc-tests (four existing ignored), strict Clippy, fmt, docs,
705-task roadmap and six production raster checks pass. The unchanged real-Poky
validator accepts failed-candidate provenance and rejects its 1.0335% CPU result.
Workspace-tested binary (preserved before docs):
/home/bspguy-dev/.local/state/yoctui-v88-validated.fbKgcY/yoctui,
SHA-256 8dfbdef0d752a7f20049bffd0f7c3f9552708ccd2f067e9f6a3b9531703d6709.
This debug baseline candidate is distinct from the preserved release measurement.

Required baseline: cargo fmt --all --check;
cargo test --workspace --all-features;
cargo clippy --workspace --all-targets --all-features -- -D warnings;
python3 -m pytest bridge/tests; ./scripts/check-docs.sh;
./scripts/verify-roadmap.sh. Runtime task additionally requires
./scripts/verify-performance.sh --real-poky-evidence. Parent then runs
./scripts/verify-completion.sh; no full-completion claim until it passes.
Every commit bumps patch version, 17 goldens and six production rasters.
All Cargo commands including docs run sequentially with CARGO_BUILD_JOBS=1,
CARGO_PROFILE_DEV_DEBUG=0, CARGO_PROFILE_TEST_DEBUG=0, CARGO_INCREMENTAL=0,
RUST_TEST_THREADS=2. Preserve exact workspace-tested executable before docs
rebuilds target/debug/yoctui using default features.

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
