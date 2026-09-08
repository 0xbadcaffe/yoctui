# Current Task

**ID:** OPENBMC-TASK-IDENTITY-001
**Title:** Use consistent authoritative recipe identities for queued and worker task events
**Status:** IN_PROGRESS

Filename-derived queue names such as llvm_git, stdplus_git and gpioplus_git
create ghost rows beside actual worker PNs. Resolve queued/started/completed
identities from bounded initialized BitBake metadata, including native/multilib
variants, git versions and PN overrides. No per-event metadata parse or filename
guessing. Unresolved identity stays honest without ghost rows; preserve task
statistics and PID-scoped log/progress correlation. Add failing bridge lifecycle
fixtures and typed task reconciliation checks. Do not interrupt the real image
solely to deploy this fix; record its live recheck after a natural stopping point.

Verification: bridge tests, full workspace tests, strict Clippy, fmt, docs and
roadmap. OPENBMC-ATTACH-TIMING-001 is DONE in v0.1.80: ten focused tests,
all 1,566 workspace tests, 49 bridge tests and baseline checks pass. Its isolated
production client preserves injected terminal duration across two attachments;
the real v0.1.76 daemon correctly yields unavailable legacy timing. Earlier
build history, screen and focus survive replacement. Evidence and failed-first
regressions are recorded in docs/testing/openbmc.md.

The tested v0.1.80 binary is preserved at
/home/bspguy-dev/.local/state/yoctui-v80-validated.GDZdkr/yoctui, SHA-256
ba7bd5dc15cb886f9268daba79e1828c9860a3bff5cd3c57f86df363181f3ca3.
It has not replaced the running daemon or normal installed release.

## OpenBMC live handoff

Continue the fifth `obmc-phosphor-image` attempt through the private Yoctui
daemon. Inspect actual task transitions, logs, terminal outcome, packages and
rootfs. Register separate atomic tasks for reproducible Yoctui defects. Preserve
all Poky data. Image success has not yet occurred; do not claim completion from
submission, cached task counts or fixture-only evidence.

Verification: real image build and generated package/rootfs inspection;
./scripts/check-docs.sh; ./scripts/verify-roadmap.sh; ./scripts/verify-completion.sh.
The completion gate still requires fresh source-bound real-Poky performance
evidence and every remaining task. Use one Cargo worker, debug=0 dev/test
profiles and no incremental cache alongside OpenBMC to retain host headroom.

## Current live identity

- Source: /home/bspguy-dev/src/openbmc
- Revision: d4fd7d3f54e88e800c0284b753af68a13aabbef6
- Build: /home/bspguy-dev/src/build-openbmc-romulus
- MACHINE romulus; DISTRO openbmc-openpower; BitBake 2.19.0
- Private daemon PID 1302830, instance 7f104be5e67613b8bb221198aea44aaa
- Running image job 1; verify live identity before lifecycle actions
- Binary: /home/bspguy-dev/.local/state/yoctui-v76-validated.tWHDMp/yoctui
- Version 0.1.76; SHA-256 93515b28b1e5bd041ce9b486f376b0dde9a006b0d334d6ce5f2ff23851061d9f
- XDG config: /home/bspguy-dev/.local/state/yoctui-openbmc-validation/config
- XDG state: /home/bspguy-dev/.local/state/yoctui-openbmc-validation/state
- XDG runtime: /run/user/1000/yoctui-openbmc-validation
- Current evidence: artifacts/live-openbmc/romulus/image-build-v76-recovery-20260908.json

The image was submitted through `yoctui daemon build obmc-phosphor-image` after
initial metadata readiness, and Accepted. Last verified counters were 3842/6812;
the image remains running with no new recorded error. Root had about
29 GiB free near the fresh attach; monitor disk and available memory. OpenBMC-only
local.conf limits BB_NUMBER_THREADS=2 and PARALLEL_MAKE=-j 2, verified before
submission. rm_work is enabled, excluding the image work directory for rootfs
inspection. Scheduling stops at 15 GiB and halts at 8 GiB. The normal installed
release remains v0.1.64; do not overwrite it merely to run this validation.

## Completed job identity repair

DAEMON-JOB-IDENTITY-001 is DONE in v0.1.78. Every non-Raw supervisor constructor
requires a shared checked allocator seeded beyond retained IDs. Raw's namespace
and live ownership maps are unchanged. The initial QA/Security test reproduced
JobId(1) in both owners; six focused regressions now pass, including real private
daemon recovery retaining old jobs 1/2 while QA/Security get 3/4, and exact-owner
cancellation between two fake BitBake bridges. All 1,556 workspace tests, 49
bridge tests, strict Clippy, formatting, docs, rasters and roadmap pass.

The tested v0.1.78 binary is preserved at
/home/bspguy-dev/.local/state/yoctui-v78-validated.RaKSXe/yoctui, SHA-256
185d14449e85f0397c80c53ce1ed5f63ec04c7282c57ab5aa08ffa6d5c9740fb.
It has not replaced the running v0.1.76 daemon. Do not interrupt the image solely
for deployment, and avoid unrelated job submissions on the old daemon while
the image runs. Read-only attach and metadata inspection remain available.

## Completed counter repair

OPENBMC-SNAPSHOT-PROGRESS-001 is DONE in v0.1.76. Optional typed build_progress
survives queue/start replacement and completed-row eviction independently of
retained rows. Only Current replica authority installs it. Five regressions
cover fresh/replacement/batched consumption, eviction, duplicate completion,
reset, legacy absence, malformed fields, unknown totals and terminal outcomes.
The first regression failed at (1, None) versus (2340, Some(6812)). All baseline
checks pass: workspace tests, strict Clippy, formatting, 49 bridge tests,
documentation, rasters and roadmap. The initial workspace run hit the existing
PTY resize race; retry with RUST_TEST_THREADS=2 passed without changing tests.

Fresh 160x50 production attach displays 2854/6812, matching aggregate/job
counters in snapshots 115 and 146, while retained task statistics contain only
1731/6812. Evidence and limits are in [OpenBMC validation](testing/openbmc.md).

## Recovery history

The second attempt reached 4953/6812 without terminal success before global host
OOM kills and user-session shutdown on September 8. The third attempt failed
linking util-linux because more-more.o was zero bytes. After Yoctui cancellation,
that object was preserved at
/home/bspguy-dev/.local/state/yoctui-openbmc-object-recovery.plNE35/more-more.o.
The fourth attempt rebuilt a valid object/main symbol and passed util-linux.
It reached 2854/6812 before explicit recovery cancellation (235 ms acknowledgement,
terminal failure recorded). After workers and daemon stopped, 23 old empty
compiled objects from function2 (8), Boost (7) and fmt (8) were moved to
/home/bspguy-dev/.local/state/yoctui-openbmc-object-recovery-23.bbZM2i with their
relative paths. They are recoverable. No empty objects remained in those three
build trees; sources, sysroots and Poky were not changed.

All earlier required capability/startup/inventory/CLI/tool-discovery fixes,
HOST-PYTHON-001, OPENBMC-ATTACH-001 and README-HEADER-001 remain DONE. Historical
versions and evidence are retained in docs/testing/openbmc.md and
docs/implementation-status.md. Do not confuse cancelled attempts or old daemon
identities with the current build.
