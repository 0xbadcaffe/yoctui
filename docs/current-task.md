# Current Task

**ID:** OPENBMC-STARTUP-001
**Title:** Keep daemon startup responsive during large initial metadata scans
**Status:** IN_PROGRESS

Depends on OPENBMC-CAPABILITY-001 (DONE in v0.1.67). OpenBMC revision
`d4fd7d3f54e88e800c0284b753af68a13aabbef6` is initialized for Romulus;
an isolated v0.1.67 daemon has Current environment authority and positive direct
build/cancel/native-event capabilities. See [OpenBMC evidence](testing/openbmc.md).

The initial storage blocker was resolved without deleting Poky: after tests,
the candidate binary was saved and regenerable Cargo workspace debug artifacts
were cleaned (79.1 GiB). Root now has 78 GiB free. Preserve all Poky data.
Establish a disk stop margin and monitor usage; this budget is not a guarantee
that the full image will fit.

Observed defect: initial full recipe inventory blocks the daemon IPC loop.
The real OpenBMC scan exceeded the 180-second readiness deadline; the parent
exited while its daemon and parser children continued. Daemon status then
reported no runtime record despite the process remaining alive. The scan later
completed, and direct socket doctor confirmed capability resolution works.
Make metadata startup nonblocking or defer it through an owned workflow while
preserving usable recipe navigation, authority, cancellation and cleanup.
Add slow-scan/lifecycle tests before the smallest coherent implementation.

Next, OPENBMC-CLI-BUILD-001 fixes repeated StaleGeneration rejections and the
incorrect zero exit status; OPENBMC-TOOLS-001 fixes executable discovery through
OpenBMC's symlinked scripts directory without weakening file-alias safety. Then execute
OPENBMC-LIVE-001 through Yoctui. No image build has started successfully yet.

Verification: focused daemon startup/lifecycle tests, full
workspace/strict Clippy, docs/roadmap and live authoritative capability recheck.
Full completion also requires fresh source-bound
real-Poky performance evidence for the changed candidate. Installed Yoctui
remains v0.1.64; no new release or successful full completion is claimed.
