# Current Task

## Task

**ID:** DAEMON-001
**Title:** Complete persistent Yoctui daemon session architecture
**Status:** DONE

## Objective

Parent gate: daemon-owned BitBake/jobs/PTYs survive client and SSH disconnect,
safely support clients/input ownership/keyboard/mouse, enforce limits/security,
state honest reboot semantics, and pass real Poky acceptance.

## Verification

```bash
./scripts/verify-completion.sh
./scripts/live-daemon-poky.sh
```

The daemon-owned BitBake build path and live validation harness are complete.
With the host namespace prerequisite enabled, the final real Poky scarthgap
`core-image-minimal` acceptance run passed fresh clone and initialization,
daemon start, detached submission, repeated reconnects, and terminal job-state
reporting. All 4567 tasks succeeded, including kernel and image creation; 3648
were reused from the shared cache. The completion-gate regression now gives
every `daemon_persist` fixture a PID-plus-monotonic temporary identity; five
consecutive parallel integration runs pass. Server IPC now separates short read
slices from bounded multi-second snapshot writes; the real PTY integration
passes ten consecutive runs. Every daemon-state fixture now has a
PID-plus-monotonic identity; all six cases, including both SSH acceptance names,
pass ten consecutive parallel runs. The next full gate reached the UI suite and
found that Configuration action guidance displaced earlier provenance at
100x25. Authoritative values, provenance, overrides, and operations now render
first, followed by compact action state and exact reasons; all 121 UI tests pass.
The next full gate then hung in `scripts/test-terminal.sh`: its piped `q` was not
observed by the pseudo-terminal application and the script had no deadline.
Navigator/Inspector now preserve global `q`/Ctrl+C routing, and the synchronized
bounded real-terminal probe passes ten consecutive runs.

The previous completion run passed the workspace, lint, terminal, fuzz, stress,
sanitizer, coverage, dependency-policy, Python, Valgrind, and optimized-profile
stages before host sampling policy blocked the required Flamegraph refresh. The
operator temporarily enabled userspace sampling on 2026-08-15;
`./scripts/flamegraph.sh` then captured real samples and regenerated the SVG.

The resumed complete gate reached the `yoctui-bitbake` library suite and exposed
a cancellation race in
`bitbake_cli_control::tests::cli_control_cancels_the_owned_process_group`: the
parallel run did not return the expected graceful `Cancelled` outcome. The task
was reopened. The fixture now publishes readiness only after installing its TERM
trap, so cancellation no longer races setup or a same-length sleep deadline.
The focused test passes 100 consecutive runs and all 180 `yoctui-bitbake`
library tests pass.

After the completion gate passes, run `./scripts/live-daemon-poky.sh` only if a
new live acceptance result is required; the existing fresh Poky scarthgap run
completed all 4567 tasks with repeated reconnects. No other registry task is
eligible; this is the terminal handoff after all registry tasks completed.
