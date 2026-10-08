# Performance limits

Measure an optimized binary of the exact source; fixtures/profiles/old captures
cannot certify current live-build performance.

## CPU accounting and release target

CPU = delta(utime+stime) / _SC_CLK_TCK / monotonic seconds ×100, from /proc/PID/stat.
1.00% is one percent of one logical CPU. Sum matching daemon/client samples.
Normal target: 10% trimmed-mean CPU, at most 1.00% of one logical CPU combined;
discard the lowest/highest 10%. One-second spikes are diagnostic.

Use Linux, a fixed 160x50 PTY, default preferences, exact release binary,
10-second warmup, and 60 one-second samples. Record revision/binary/toolchain,
PID/daemon, host/kernel/CPU/affinity/RAM/filesystem/free space/load and dimensions.
Changed PID, missing metadata/window, or exit invalidates the run. Startup has
an eight-second first-frame limit, measured separately.

Measure real attach startup with a private session, without restarting the daemon:

```bash
python3 scripts/measure-attach-startup.py --binary target/release/yoctui
```

This records time to the first composed workspace, binary SHA256, daemon
connection and inotify watch count at 160x50; graphics probing/desktop handoff
are disabled for a repeatable comparison. It signals only its owned client and
leaves the existing daemon/builds/terminals untouched. Report traced timing
separately: syscall tracing significantly inflates filesystem traversal time.

### Attach fix in 0.1.319

On the existing OpenBMC daemon, controlled warm-cache first frames improved
from 3.52 seconds (installed 0.1.315) to 0.035–0.041 seconds; the normal terminal
graphics probe took 0.48 seconds. Watches fell from 64,889 to 4,101. The previous
watcher implementation also matches the 0.1.318 branch base. Syscall tracing
identified 60,179 watches inside generated build output before the fix and zero
afterward; its 41.7-second old startup is diagnostic, not an unprofiled result.
Setup now runs asynchronously with bounded tracked-directory watches instead
of a recursive filesystem walk. Existing event refresh and 30-second fallback
remain; builds and terminal sessions do not need to restart.
See the [measurement receipt](performance/attach-startup-0.1.319.json) for exact
binary/source hashes, installed-client measurement, methods and focused checks.

## Scenario thresholds

| Scenario | Workload | CPU ceiling |
| --- | --- | --- |
| Idle daemon | No clients/jobs/PTYs/probes. | daemon <=0.20% |
| Idle attached client | Dashboard/clock, no input/build/PTY. | client <=0.50%, combined <=1.00% |
| Active build | 50–200 task/log events/s. | combined <=1.00% |
| PTY attached but idle | Writer, 120x40 idle prompt. | combined <=1.00% |
| High-rate BitBake stream | Required deterministic mix, 2,000 events/s. | combined <=5.00% |
| Two attached clients | Two idle 160x50 clients. | total <=1.50%, extra client <=0.50% |

First four establish normal CPU; floods also need latency/continuity/memory checks.

## Responsiveness and rendering

All allowed CPUs runnable; at least 100 post-warmup observations:

- key press to reducer action p95 <=100 ms;
- key press to visible frame p95 <=100 ms;
- mouse event to visible selection p95 <=100 ms;
- daemon event to client receipt p50 <=25 ms and p95 <=100 ms;
- client command to daemon receipt p50 <=25 ms and p95 <=100 ms;
- cancellation request to daemon acknowledgement p95 <=250 ms.

Input cannot wait for redraw. Normal dirty/animated views: 4 Hz, 250 ms normal frame interval; saturated-build cosmetics (host >=90%)/clock: 1 Hz; active PTY
publication <=30 frames/s. Hidden activity does not redraw; reduced motion freezes
animation. Discovery is lower priority; active work retains the
35 ms supervisor-service bound.

## IPC and BitBake liveness under saturation

A peer dies only when three consecutive replies are absent over at least 90 seconds;
any decoded message establishes liveness. Scheduling/read delay alone is not loss.
Reconnect backs off; expired replay requests a snapshot.

Slow clients cannot block ingestion/other clients. Failure/error/cancellation/
terminal/capability/warning/task-lifecycle/input/PTY control-output records are
never dropped. Progress/telemetry coalesces between ordering barriers; ordinary
logs/cosmetics expose loss. Coalescing cannot invent success.

## Memory and resource bounds

For 30 minutes at high rate: daemon RSS growth after warmup <=32 MiB;
client RSS growth after warmup <=32 MiB; final-20-minute positive slope <=64 KiB/min;
no thread growth; all retention/queue/scrollback/telemetry bounds hold.
Record RSS/virtual memory/threads/context switches/wakeups/frame/IPC/event rates
and queue pressure. Short fixtures cannot establish this endurance result.

## Baseline and profiling artifact policy

Generated outputs: artifacts/performance/baseline/, results/, profiles/.
Record scenario/method/command/source/binary/host/window/statistic/thresholds/hashes.
Profiles cover daemon/client/build/log/task/PTY cases with resolved symbols;
acceptance CPU is unprofiled. Real-Poky evidence includes exact release/revisions,
machine/distro/config/target/task interval/parallelism and measurements.
Generators remain fixture evidence; partial intervals are not completed images.

## Commands

| Check | Command |
| --- | --- |
| CPU | `./scripts/verify-low-overhead.sh` |
| Saturation | `./scripts/verify-saturation-responsiveness.sh` |
| IPC | `./scripts/verify-ipc-continuity.sh` |
| Memory | `./scripts/verify-bounded-memory.sh` |
| Process profile | `./scripts/capture-runtime-profile.sh` |
| Live build | `./scripts/capture-real-poky-performance.py` |
| Source-bound regression | `./scripts/verify-performance.sh --regressions` |
| CI policy | `./scripts/verify-performance.sh --ci` |

Use --help for focused modes. Saturation uses full allowed affinity with reaped
owned workers; smaller explicit diagnostics do not satisfy that gate.
No supported path requires root, real-time scheduling, or reserved CPUs. Tuning
cannot change others' processes; BitBake parallelism is user-owned—Yoctui never changes them.

## Evidence and CI

Compare matching methods/scenarios/hosts; source-bound hashes and correctness
sentinels must verify. Missing/stale external input fails as a prerequisite.
PR checks cover event loops/render/coalescing/saturation/IPC, not image completion
or 30-minute endurance. Scheduled/manual jobs measure fresh CPU/memory;
real-Poky opt-in uses YOCTUI_LIVE_PERFORMANCE=1, YOCTUI_PERF_BUILD_DIR,
YOCTUI_PERF_POKY_ROOT. See [Testing](testing.md) and [Profiling](profiling.md).
