# v0.1.89 completion-gate failures

These are failed verification diagnostics, not passing performance evidence.
Runtime sources are commit `11f3d8f` (v0.1.89).

## Obsolete constructor assertion

The exact clean worktree
`/home/bspguy-dev/.local/state/yoctui-v89-completion.QlFQ43` ran
`./scripts/verify-completion.sh`; session 99674 exited 1. Full log:
`/tmp/yoctui-v89-completion.log`.

Before failure, workspace tests, strict Clippy, the five UI performance
scenarios (0.505–1.096 ms/frame), source-bound real-Poky evidence and the
22-hard-metric/seven-correctness regression record passed. Fast performance CI
then invoked `verify-ipc-continuity.sh --backpressure`. Its inline Python
source checker split on `impl Default for DaemonBitBakeSupervisor`, which no
longer exists after shared job IDs moved construction to `new(job_ids)`.
The unchecked `[1]` access raised `IndexError` before checking actual ingress.

Five source-checker regression groups reproduce this on the original checker
(11 errors including subtests); log `/tmp/yoctui-v90-ipc-failed-first.log`.
They pass after inspecting the explicit constructor and requiring exact bounded
reliable, cosmetic and cancellation-terminal channel assignments. Production
queue code is unchanged.

## Premature event-flood build request

With only the checker repaired, `./scripts/verify-ipc-continuity.sh
--backpressure` passed its source checks, three focused Rust tests, retained
evidence checks and four fixture tests, then exited 1:
`RuntimeError: event generator did not publish its bounded report`.
Log: `/tmp/yoctui-v90-ipc-backpressure.log`.

A separate diagnostic wrapped only `ProtocolClient.receive` to print received
frames, leaving the actual fixture, command sequence and timing unchanged.
It reproduced the same failure. Log: `/tmp/yoctui-v90-flood-diagnostic.log`.
The exact binary was preserved before subsequent Cargo output could replace it:

- Path: `/home/bspguy-dev/.local/state/yoctui-v89-flood-diagnostic.ymsiXE/yoctui`
- Version: `yoctui 0.1.89`
- SHA-256: `abbed425952c112633fdeace40ab95acb85cd37e59656bdf12f47013bb66657b`

Observed protocol order:

1. Both client attachments had generation 3 and no retained workspace.
2. Request 1, `start_build`, was rejected with code `conflict`, current
   generation 3: `Initial recipe inventory is still loading; retry the build
   when metadata is ready`.
3. Sequence/generation 4 published the fixture's workspace event.
4. Sequence/generation 5 published daemon-metadata log
   `Initial workspace and recipe inventory ready`.
5. Subsequent telemetry reported zero active jobs; the generator never started.

The daemon correctly enforced startup readiness. The fixture did not wait for
it or fail immediately on the rejected command. Each disposable daemon was
stopped by the harness's existing cleanup; no original Poky/OpenBMC build or
normal installed daemon was touched. RELEASE-FIXTURE-READY-001 is registered
before harness changes and affected source-bound evidence refresh. Neither the
full backpressure gate nor repository completion has passed.
