# Current Task

**ID:** OPENBMC-ATTACH-001
**Title:** Allow a healthy large daemon snapshot to complete initial client attachment
**Status:** IN_PROGRESS

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
daemon is now PID 3955092, with no active build. Verify identity before stopping.

Newly observed: initial client attachment uses a 250 ms timeout and can reject
a healthy large Romulus snapshot, then initialize the UI/local adapters as if
no daemon environment exists. Give only pre-terminal attachment a five-second
receive budget. Keep interactive reconnect waits short. Add a delayed socket
regression first, check missing-socket behavior and installed workspace identity,
then recheck the real client. The delayed snapshot test failed at 250 ms and
passes at five seconds. The client-only implementation is not yet committed;
finish the missing-socket test, complete checks, version bump and live capture.
Verify identity before stopping. Private XDG paths are in the evidence.
Source revision d4fd7d3f54e88e800c0284b753af68a13aabbef6, source
/home/bspguy-dev/src/openbmc, build /home/bspguy-dev/src/build-openbmc-romulus,
MACHINE romulus, DISTRO openbmc-openpower, BitBake 2.19.0.

Execute OPENBMC-LIVE-001 through Yoctui: build
obmc-phosphor-image, inspect lifecycle/logs/outcomes, packages and rootfs, and
register/fix real regressions. The image was accepted; no success is claimed yet.
Preserve all Poky data. Root has about 67 GiB free after cleaning agent Cargo
outputs and rebuilding. New OpenBMC guards stop scheduling at 15 GiB / halt at 8 GiB.
Inspect the shallow checkout's os-release Git-tag warning during validation.

Verification: real image build and inspection, docs/roadmap checks, followed by
the full completion gate. Installed
release remains v0.1.64. Full completion still requires fresh source-bound
real-Poky performance evidence and all remaining tasks, not fixture-only claims.
