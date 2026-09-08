# Current Task

**ID:** DAEMON-JOB-IDENTITY-001
**Title:** Keep daemon job identities unique across supervisors and recovered history
**Status:** IN_PROGRESS

The v0.1.76 restart recovered failed jobs 1 and 2, then the new BitBake build
reused ID 1 and replaced its history. Independent supervisor counters also
collide across operation types. Introduce one checked shared allocator for
non-Raw daemon jobs, seeded beyond recovered IDs and disjoint from Raw's
namespace. Fail closed before spawning work on exhaustion. Cover recovery,
cross-supervisor identity and exact cancellation with isolated process/bridge
tests. Preserve the running real image; no restart solely for deployment.

Verification: cargo test -p yoctui daemon_job_identity; baseline workspace
tests, Clippy, fmt, bridge tests, docs and roadmap. Update architecture/UI/
protocol, registry, implementation status and OpenBMC evidence. After the fix,
return to OPENBMC-LIVE-001 and investigate remaining observed UI discrepancies.

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
initial metadata readiness, and Accepted. Last verified counters were 2854/6812;
LLVM native compilation and Boost installation were running. Root had about
35.6 GiB free at the fresh attach; monitor disk and available memory. OpenBMC-only
local.conf limits BB_NUMBER_THREADS=2 and PARALLEL_MAKE=-j 2, verified before
submission. rm_work is enabled, excluding the image work directory for rootfs
inspection. Scheduling stops at 15 GiB and halts at 8 GiB. The normal installed
release remains v0.1.64; do not overwrite it merely to run this validation.

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
