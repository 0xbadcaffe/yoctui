# Current Task

**ID:** OPENBMC-ENV-001
**Title:** Provision an isolated supported OpenBMC machine workspace
**Status:** IN_PROGRESS

Depends on COMPACT-TELEMETRY-001 (DONE in the v0.1.65 candidate).
Compact meters and shared task mouse geometry pass 1,520 workspace tests,
strict Clippy, formatting, 46 bridge tests and documentation checks.
See [compact telemetry evidence](testing/compact-telemetry.md).

The initial storage blocker was resolved without deleting Poky: after tests,
the candidate binary was saved and regenerable Cargo workspace debug artifacts
were cleaned (79.1 GiB). Root now has 78 GiB free. Preserve all Poky data.
Establish a disk stop margin and monitor usage; this budget is not a guarantee
that the full image will fit.

Clone OpenBMC separately, record its revision, initialize Romulus, verify
Yoctui doctor in that environment, and proceed to OPENBMC-LIVE-001 to build
`obmc-phosphor-image` through Yoctui and investigate actual integration bugs.
See [OpenBMC preflight](testing/openbmc.md). No OpenBMC build has run yet.

Verification: `./scripts/check-docs.sh`, `./scripts/verify-roadmap.sh`, live
machine/environment evidence. Full completion also requires fresh source-bound
real-Poky performance evidence for the changed candidate. Installed Yoctui
remains v0.1.64; no new release or successful full completion is claimed.
