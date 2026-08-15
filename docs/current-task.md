# Current Task

## Task

**ID:** BRIDGE-PROGRESS-001
**Title:** Normalize live BitBake progress and render task bars
**Status:** IN_PROGRESS

## Objective

Keep live Poky builds connected when BitBake emits fractional process progress,
correlate PID-only task-progress events with their authoritative task-start
identity, and render determinate per-task progress bars without fabricating
progress when BitBake reports an unknown or invalid value.

The implementation and task-specific verification are complete. The installed
binary connected to `/home/bspguy-dev/src/poky/build-yoctui` through BitBake
2.8.1 and reported the Scarthgap 5.0.19 workspace without a JSON protocol error
or reconnect loop. All 39 bridge tests, workspace tests, Clippy, documentation,
and roadmap checks pass.

## Resumed verification

The operator enabled the required temporary perf permission on 2026-08-15, so
the repository completion gate is running again.

## Verification

```bash
sudo sysctl -w kernel.perf_event_paranoid=0
./scripts/verify-completion.sh
sudo sysctl -w kernel.perf_event_paranoid=4
```

After the gate passes, restore `kernel.perf_event_paranoid=4`, change this task
to `DONE`, update the implementation status, retain this final completed task
as the terminal handoff, and commit the completion evidence.
