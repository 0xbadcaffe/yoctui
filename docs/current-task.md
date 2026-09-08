# Current Task

**ID:** RELEASE-PERF-REFRESH-001
**Title:** Refresh source-bound real-Poky performance evidence after OpenBMC integration
**Status:** IN_PROGRESS

v0.1.87 registration passes all 1595 workspace tests/doc-tests, 52 bridge tests,
strict Clippy, fmt, docs, roadmap and version-policy checks/tests. All 17 golden
diffs are version digits only and six rasters verify. The
workspace-tested candidate is preserved at
/home/bspguy-dev/.local/state/yoctui-v87-validated.XKyMwk/yoctui,
SHA-256 e0736f32129210142a9409047047eea62f206e71f10761d508c9f485c5ba640c.
The OpenBMC daemon remains the separately validated v0.1.86 d24d candidate.
No new performance fixture has been initialized yet.

OPENBMC-LIVE-001 is DONE in v0.1.87. All integration repairs are DONE, including
v0.1.86 exact-image daemon
rootfs source acquisition and v0.1.85 runtime-reverse package identities.
The real Romulus image SUCCEEDED: 6812/6812, image job 1 exit 0 and successful
typed terminal. Generated flash/SquashFS artifacts, 228 packages, retained
rootfs and services were inspected. All observed Yoctui integration findings
now have tested repairs and actual live rechecks in docs/testing/openbmc.md.
Refresh actual supported Poky 6.0.2 linux-yocto do_compile release performance
evidence, then run ./scripts/verify-completion.sh. The exact real-Poky validator
currently rejects historical v64 source hashes at crates/yoctui-app/src/lib.rs.
Use a new isolated fixture with its own writable TMPDIR and SSTATE_DIR, bounded
disk/memory headroom and existing caches only as read sources. Daemon-owned
cleansstate is permitted only for this newly created fixture, never existing
Poky builds or old performance fixtures. Preserve historical evidence; update
manifest/source provenance only from actual measured release-binary output.
Keep all saturation/CPU/latency/continuity/cancellation thresholds unchanged.
Do not run Cargo builds during the measured saturation window.
Do not claim full completion until that gate passes. Fresh source-bound
real-Poky performance evidence is still required; preserve all existing Poky
data. Investigate safe alternatives before declaring an external blocker.

## Verification and boundaries

v0.1.86 passes ten focused CLI rootfs tests, two protocol tests and one
full-instance app test; all 1595 workspace tests/doc-tests, 52 bridge tests,
strict Clippy, fmt, docs, version-policy checks/tests, 17 version-only goldens
and six production rasters. Real source acquisition returns exact retained
rootfs/pkgdata without job-history mutation. Actual filesystem totals match
an independent scan: 2628 entries, 81096497 bytes. Installed package metadata
and offline system inventory are available; real services/previews work.
Filesystem package ownership remains explicitly unknown, an intentional
partial limitation distinct from acquisition/traversal failure.

Verification: ./scripts/verify-performance.sh --real-poky-evidence;
./scripts/check-docs.sh; ./scripts/verify-roadmap.sh;
./scripts/verify-completion.sh and baseline cargo fmt --all --check;
cargo test --workspace --all-features; cargo clippy --workspace --all-targets
--all-features -- -D warnings; python3 -m pytest bridge/tests.
Every commit bumps patch version, 17 goldens and six rasters. Run all Cargo
commands including docs sequentially with one worker, dev/test debug=0,
no incremental and RUST_TEST_THREADS=2. Preserve the exact workspace-tested
binary before docs rebuilds the default-feature target/debug executable.

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
