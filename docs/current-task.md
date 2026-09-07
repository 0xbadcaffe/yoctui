# Current Task

**ID:** OPENBMC-LIVE-001
**Title:** Build one OpenBMC machine through Yoctui and investigate integration defects
**Status:** IN_PROGRESS

All five required integration fixes are DONE through v0.1.71: direct capability
probing, responsive startup, bounded inventory, CLI stale retries/rejections,
and symlinked PATH tool discovery. All 1,543 workspace tests, 49 bridge tests,
strict Clippy, formatting, Python checks and docs pass. See
[OpenBMC validation](testing/openbmc.md).

Private OpenBMC v0.1.71 daemon PID 2499442 is idle with no build job.
Verify identity before stopping. Private XDG paths are in the evidence.
Source revision d4fd7d3f54e88e800c0284b753af68a13aabbef6, source
/home/bspguy-dev/src/openbmc, build /home/bspguy-dev/src/build-openbmc-romulus,
MACHINE romulus, DISTRO openbmc-openpower, BitBake 2.19.0.

Execute OPENBMC-LIVE-001 through Yoctui: build
obmc-phosphor-image, inspect lifecycle/logs/outcomes, packages and rootfs, and
register/fix real regressions. No image build has been accepted yet.
Preserve all Poky data. Root has about 67 GiB free after cleaning agent Cargo
outputs and rebuilding. New OpenBMC guards stop scheduling at 15 GiB / halt at 8 GiB.
Inspect the shallow checkout's os-release Git-tag warning during validation.

Verification: real image build and inspection, docs/roadmap checks, followed by
the full completion gate. Installed
release remains v0.1.64. Full completion still requires fresh source-bound
real-Poky performance evidence and all remaining tasks, not fixture-only claims.
