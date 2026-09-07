# Current Task

**ID:** OPENBMC-LIVE-001
**Title:** Build one OpenBMC machine through Yoctui and investigate integration defects
**Status:** IN_PROGRESS

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
stalled it; no image success is claimed. The corrected-environment private
daemon is PID 3955092. Retry job 1 is running and has passed 2,052/6,812 tasks;
no image completion is claimed. Verify identity before stopping.

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
Preserve all Poky data. Root had about 53 GiB free at the README handoff after
preserving v0.1.73 and cleaning regenerable agent Cargo outputs; monitor growth.
New OpenBMC guards stop scheduling at 15 GiB / halt at 8 GiB.
The shallow checkout's os-release warning was resolved by fetching history/tags
without changing HEAD. rm_work is enabled only in OpenBMC, excluding the image
work directory so rootfs inspection remains possible. Parallelism is unchanged.

Verification: real image build and inspection, docs/roadmap checks, followed by
the full completion gate. Installed
release remains v0.1.64. Full completion still requires fresh source-bound
real-Poky performance evidence and all remaining tasks, not fixture-only claims.
