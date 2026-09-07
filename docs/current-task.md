# Current Task

**ID:** OPENBMC-CAPABILITY-001
**Title:** Probe actual backend capabilities before resolving newer BitBake environments
**Status:** IN_PROGRESS

Depends on OPENBMC-ENV-001 (DONE). OpenBMC revision
`d4fd7d3f54e88e800c0284b753af68a13aabbef6` is initialized for Romulus;
an isolated v0.1.65 daemon has Current environment authority and a working
bridge handshake. See [OpenBMC evidence](testing/openbmc.md).

The initial storage blocker was resolved without deleting Poky: after tests,
the candidate binary was saved and regenerable Cargo workspace debug artifacts
were cleaned (79.1 GiB). Root now has 78 GiB free. Preserve all Poky data.
Establish a disk stop margin and monitor usage; this budget is not a guarantee
that the full image will fit.

Observed defect: `DaemonCompatibilityRuntime::detect` always supplies `None`
for backend capabilities, leaving BitBake 2.19 build operations unknown outside
the closed version fallback. Implement bounded read-only capability probing
with real API evidence, tests first. Preserve unknown on inconclusive probes;
do not widen the version map or bypass authority to make this build run.

Next, OPENBMC-CLI-BUILD-001 fixes repeated StaleGeneration rejections and the
incorrect zero exit status of rejected `daemon build` commands. Then execute
OPENBMC-LIVE-001 through Yoctui. No image build has started successfully yet.

Verification: bridge tests, focused daemon compatibility/backend tests, full
workspace/strict Clippy, docs/roadmap and live authoritative capability recheck.
Full completion also requires fresh source-bound
real-Poky performance evidence for the changed candidate. Installed Yoctui
remains v0.1.64; no new release or successful full completion is claimed.
