# Current Task

**ID:** OPENBMC-SNAPSHOT-PROGRESS-001
**Title:** Preserve authoritative build counters across compacted daemon snapshots
**Status:** IN_PROGRESS

Live attachment exposed a reproducible progress discrepancy: daemon job progress
was 2,339/6,812 while the retained build snapshot kept only an older 1,731-task
statistic and eight completed rows. Snapshot compaction removes the newer
statistics together with completed task start/queue events. Preserve bounded
typed aggregate counters independently of retained task rows; fresh attachment
and replacement snapshots must agree with uninterrupted event consumption.
Keep unknown totals unknown and reset authority between builds. Add focused
protocol/app and TestBackend regressions, run baseline checks, and return to
OPENBMC-LIVE-001 after verification. Do not interrupt the current image merely
to deploy the candidate; record its separate binary/version and live recheck.

Verification: cargo test -p yoctui-protocol snapshot_progress;
cargo test -p yoctui-app snapshot_progress; cargo test -p yoctui-ui snapshot_progress;
cargo fmt --all --check; cargo test --workspace --all-features;
cargo clippy --workspace --all-targets --all-features -- -D warnings;
python3 -m pytest bridge/tests; ./scripts/check-docs.sh; ./scripts/verify-roadmap.sh.
Use bounded Cargo compilation concurrency alongside the OpenBMC build.

## OpenBMC live handoff

README-HEADER-001 is DONE in v0.1.74: supplied artwork adapted into a compact
header with accurate clickable badges and navigation. Static README/link,
version, formatting, locked metadata and raster/gallery checks pass. No runtime
source changed or release gate rerun. Resume the existing OpenBMC validation.

All five required integration fixes are DONE through v0.1.71: direct capability
probing, responsive startup, bounded inventory, CLI stale retries/rejections,
and symlinked PATH tool discovery. All 1,543 workspace tests, 49 bridge tests,
strict Clippy, formatting, Python checks and docs pass. See
[OpenBMC validation](testing/openbmc.md).

HOST-PYTHON-001 is DONE: persistent default shells use system Python, pyenv
2.8.5 prevents shim-alias loops, both managed interpreters are preserved and
inactive OpenBMC hosttools/python3 is repaired. Shell, restricted-PATH and
managed-version tests pass; backup and scope in testing/host-python.md.
OpenBMC job 1 was cancelled with 282 ms acknowledgement after host recursion
stalled it; no image success is claimed. The corrected-environment retry reached
4,953/6,812 tasks in its saved capture, without a terminal result. On September 8
the old daemon PID 3955092 and all workers were absent. The kernel recorded host
OOM kills at 03:50 and the user session then shut down; this is not image success.
The third attempt was accepted through the preserved v0.1.73 candidate after
limiting the isolated OpenBMC build to two BitBake tasks and two compiler jobs.
Its private daemon is PID 1195382, instance eceb92e7253969e488f7156c31d35d0f.
Verify live identity before stopping. The third attempt failed util-linux linking
because text-utils/more-more.o was zero bytes, then was cancelled through Yoctui
(terminal failure retained). Only that object was moved to a recoverable backup
at /home/bspguy-dev/.local/state/yoctui-openbmc-object-recovery.plNE35/more-more.o.
The fourth attempt is job 2 in the same daemon instance. It rebuilt more with
the main symbol and completed util-linux compilation and subsequent package tasks.
Current bounded evidence is
artifacts/live-openbmc/romulus/image-build-object-recovery-20260908.json.

OPENBMC-ATTACH-001 is DONE in v0.1.73: initial snapshot reads get five seconds,
with separate 250 ms discovery and in-loop reconnect limits. Both delayed and
missing-socket regressions pass; all 1,545 workspace tests, 49 bridge tests,
strict Clippy, formatting, Python and docs checks pass. Real attached capture
without local build env opens Dashboard directly at 140x32 in 1.336 seconds.
Continue the image through the corrected-environment daemon and inspect actual
task/log/output behavior. Keep runtime evidence distinct from release performance.
Verify identity before stopping. Private XDG paths are in the evidence.
Source revision d4fd7d3f54e88e800c0284b753af68a13aabbef6, source
/home/bspguy-dev/src/openbmc, build /home/bspguy-dev/src/build-openbmc-romulus,
MACHINE romulus, DISTRO openbmc-openpower, BitBake 2.19.0.

Execute OPENBMC-LIVE-001 through Yoctui: build
obmc-phosphor-image, inspect lifecycle/logs/outcomes, packages and rootfs, and
register/fix real regressions. The image was accepted; no success is claimed yet.
Preserve all Poky data. Root had about 46 GiB free at the September 8 restart;
monitor disk growth and available memory. The preserved candidate is
/tmp/yoctui-v73-validated.vMBk6H/yoctui, SHA-256
f34b9b26fb6d60d1503b20674bf194252552159ca63953b37000eb3e0b817a26.
New OpenBMC guards stop scheduling at 15 GiB / halt at 8 GiB.
The shallow checkout's os-release warning was resolved by fetching history/tags
without changing HEAD. rm_work is enabled only in OpenBMC, excluding the image
work directory so rootfs inspection remains possible. Only OpenBMC local.conf
now sets BB_NUMBER_THREADS = "2" and PARALLEL_MAKE = "-j 2"; both were verified
with bitbake-getvar before submission. Source revision and Poky remain unchanged.

Verification: real image build and inspection, docs/roadmap checks, followed by
the full completion gate. Installed
release remains v0.1.64. Full completion still requires fresh source-bound
real-Poky performance evidence and all remaining tasks, not fixture-only claims.
