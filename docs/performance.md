# Low-Overhead and Build-Saturation Performance Contract

This document is the normative M46 measurement and responsiveness contract.
Performance results are valid only when the scenario record contains every
field required below. Faster deterministic fixtures never substitute for the
separately required real-Poky evidence.

## CPU accounting and release target

Linux process CPU is measured from fields 14 and 15 (`utime + stime`) in
`/proc/<pid>/stat`. The delta in clock ticks is divided by `_SC_CLK_TCK` and by
the monotonic wall-clock sample duration, then multiplied by 100. Daemon and
client percentages are calculated independently and added for the combined
result. This is normal process accounting: 1.00% means one percent of one
logical CPU on every host, not one percent of total machine capacity.

The release goal for the combined daemon plus one attached interactive client
is a 10% trimmed-mean CPU of **at most 1.00% of one logical CPU** in the
steady-state normal-operation scenarios. The first and last 10% of ordered
samples are discarded before the arithmetic mean. Individual one-second
samples are diagnostic and never decide the gate.

## Controlled host and measurement window

The controlled release gate uses Linux, a fixed 160x50 PTY, Unicode/color
enabled, reduced motion disabled, the default configured refresh of 100 ms,
and a release build of the exact checked-out revision. It records UTC time,
kernel, CPU model, logical-CPU count, online CPU set, governor when available,
RAM, filesystem type/free space, terminal dimensions, Yoctui commit/binary
hash, daemon instance, Rust version, and other host load.

Each steady-state scenario has a 10-second warmup followed by 60 one-second
samples. Startup may exceed the steady-state target only during a bounded
five-second startup interval; startup is excluded from the 10-second warmup
and remains subject to the existing eight-second first-frame gate. A result is
invalid if a PID changes, a process exits, the monotonic sample window is
short, or host metadata is absent. CPU accounting uses `CLOCK_MONOTONIC` for
windows and `/proc` only for process counters.

## Scenario thresholds

| Scenario | Exact steady state | CPU threshold |
| --- | --- | ---: |
| Idle daemon | initialized daemon, zero clients/jobs/PTYs, no compatibility probe in flight | daemon <=0.20% |
| Idle attached client | Dashboard/Navigator, no build, no PTY, no input, clock visible | client <=0.50%; combined <=1.00% |
| Active build | daemon-owned build with live task/log updates at 50-200 events/s | combined <=1.00% |
| PTY attached but idle | one 120x40 daemon PTY, writer attached, no output after prompt | combined <=1.00% |
| High-rate BitBake stream | deterministic 2,000 events/s burst with the required event mix | combined <=5.00%; normal-operation target does not apply |
| Two attached clients | two idle 160x50 clients on one daemon | total <=1.50%; second-client marginal <=0.50% |

Only the first four rows are steady-state normal operation for the 1.00%
release goal. The flood ceiling is a protection threshold paired with latency,
continuity, and bounded-memory gates, not permission for high idle overhead.
If multi-client support changes, the two-client scenario remains required and
its changed threshold needs an explicit contract revision.

## Responsiveness and rendering

Under a deterministic load that keeps every online logical CPU runnable:

- key press to reducer action p95 <=100 ms
- key press to visible frame p95 <=100 ms
- mouse event to visible selection p95 <=100 ms
- daemon event to client receipt p50 <=25 ms and p95 <=100 ms
- client command to daemon receipt p50 <=25 ms and p95 <=100 ms
- cancellation request to daemon acknowledgement p95 <=250 ms

Every latency series contains at least 100 observations after warmup and uses
monotonic timestamps. Keyboard processing cannot depend on a render tick.
Screen updates must continue when state changes, but identical state must not
force a redraw. Idle clock updates are at most 1 Hz; visible ordinary activity
animation is 4-10 Hz; normal build rendering is at most 10 frames/s after
coalescing; active PTY screen publication remains at most 30 frames/s.
Reduced-motion mode freezes animation. Hidden animation and telemetry cannot
invalidate the frame.

## IPC and BitBake liveness under saturation

Liveness uses monotonic time. A heartbeat is nominally sent every 30 seconds;
the peer is not declared dead until three consecutive replies are absent over
at least 90 seconds. Any successfully decoded message proves current liveness.
Socket read slices and write deadlines may bound work, but scheduler delay or
an ordinary timeout alone cannot be called a backend disconnect. Reconnect is
attempted with bounded backoff and can request a current snapshot when a replay
cursor expires.

Each client has an independently bounded outbound queue. Slow clients cannot
block BitBake ingestion or other clients. Failures, errors, cancellation,
terminal outcomes, backend disconnects, user input, capability correctness
changes, warnings, and PTY control/output are never dropped. Repeated progress,
telemetry, ordinary logs, and animation ticks may be replaced or coalesced by
stable identity while preserving the next critical event's ordering boundary.
Pressure counters report coalesced and dropped cosmetic records, maximum queue
depth, and forced resynchronizations.

## Memory and resource bounds

The baseline record includes RSS, virtual memory, threads, voluntary and
involuntary context switches, wakeups/s when the host exposes them, render
frequency, IPC messages/bytes per second, BitBake events/s, telemetry polls/s,
and queue-pressure counters. The deterministic 30-minute endurance gate uses
the high-rate event mix and requires:

- daemon RSS growth after warmup <=32 MiB
- client RSS growth after warmup <=32 MiB
- no positive least-squares RSS slope above 64 KiB/minute in the final 20 minutes
- logs, task history, journal, per-client queues, PTY scrollback, and telemetry
  history remain at their declared model/protocol bounds
- thread count does not grow after warmup

The default one-minute PR fixture may prove bounds and state invariants but is
not the endurance result used for a release claim.

## Baseline and profiling artifact policy

Pre-optimization machine-readable artifacts live below
`artifacts/performance/baseline/`; optimized gate results live below
`artifacts/performance/results/`; concise reports and flamegraphs live below
`artifacts/performance/profiles/`. Tracked evidence records schema version,
scenario, command, exact revision and binary hash, host fields, warmup/window,
raw sample count, robust statistic, thresholds, and artifact SHA-256 values.
Large raw `perf.data` captures remain reproducible local artifacts and are not
tracked.

Profiles are required for idle daemon, idle client, active build, log-heavy,
task-event-heavy, PTY-idle, and PTY-active scenarios. A report is invalid when
sampling is unavailable, unresolved frames exceed the existing quality bound,
or workload identity is missing. `perf`, cargo-flamegraph, or samply may be
used; Tokio Console is admitted only if its instrumentation overhead is
recorded and the result is not used as a CPU-gate measurement.

Real-Poky evidence must name the supported release, exact revisions,
configuration, build target, duration of sustained task execution, BitBake
parallelism, and measured Yoctui/BitBake metrics. It may end after a meaningful
sustained interval when completing the image is impractical, but must never be
labelled as fixture evidence or inferred from a deterministic generator.

## Reproduction entry points

`scripts/cpu-saturation-harness.py` is the deterministic offline CPU-load
fixture. With no worker or CPU arguments it discovers the caller's complete OS
affinity set, pins one worker to every available logical CPU, and deliberately
reserves none. `--workers` and `--cpu-list` can select a smaller explicit test
set. Each worker reports readiness before one shared start signal, runs a
separate warmup, then performs bounded deterministic integer work until the
monotonic deadline. SIGINT/SIGTERM set the shared stop signal and the parent
joins, terminates, or kills only its exact child identities before exiting.

The JSON result and optional JSON-lines event log record selected affinity,
worker PIDs, per-worker iterations/checksum/CPU time, aggregate and minimum
worker load, host utilization/load average, total elapsed time, and proof that
all children were reaped. The harness requires no network, Yocto checkout,
privilege, real-time policy, or deliberately free CPU. The fast gate runs the
full affinity set with a short bound:

```sh
./scripts/verify-saturation-responsiveness.sh --harness
```

The BitBake connection gate runs production `BridgeBackend` and daemon
supervisor fixtures while one pinned computation worker keeps every available
logical CPU runnable. A 250 ms silent native-event interval must remain
connected, actual bridge EOF must produce exactly one typed disconnect, and a
cancellation terminal must publish before deliberately hung server cleanup.
Active event reads have no elapsed-time disconnect heuristic; fixed 64-record
native batches return to command input, and cancellation fallback uses Tokio's
monotonic clock.

```sh
./scripts/verify-saturation-responsiveness.sh --bitbake-connection
```

### Tokio runtime audit

`scripts/measure-tokio-runtime.py` launches an isolated idle daemon, samples
every `/proc/<pid>/task` CPU counter and context-switch counter over a fixed
three-second audit window, records the stable thread set, and inventories Tokio
spawn, blocking-pool, and channel construction sites. The hashed evidence and
decision record live under `artifacts/performance/tokio/`.

On the eight-logical-CPU reference host, the default runtime created eight
workers plus the main thread. All workers recorded zero CPU ticks, while the
combined process recorded 0.3333% of one logical CPU. The explicit two-worker
runtime retained the same measured CPU and reduced the stable process set from
nine threads to three. No lazy blocking-pool thread existed in either idle
sample. The source audit recorded 25 `spawn_blocking` call sites; expensive
filesystem, process, compatibility, and inventory work stays behind that
boundary. Two workers are required because terminal input and daemon listener
readiness use bounded synchronous polls: one worker may be in that poll while
the other must continue the reactor, IPC, input, and monotonic timers.

The offline gate dynamically checks the current daemon still has exactly two
runtime workers and three stable idle threads. It also occupies one worker for
750 ms, then requires a second-worker timer/task response within 500 ms while
the CPU fixture keeps every CPU in the process affinity runnable:

```sh
./scripts/verify-performance.sh --tokio
```

This is runtime-scheduling evidence, not a substitute for the 60-second release
CPU scenarios or real-Poky saturation evidence.

`scripts/fixtures/bitbake-event-flood-bridge.py` is the deterministic bridge
fixture. It accepts a rate, duration, balanced/log-heavy/task-heavy profile,
and success/failure/disconnect terminal mode. Stable task identities plus
ordered critical sentinels cover ordinary logs, progress, warnings, errors,
task failures, cancellation, backend EOF, and build terminals. Its atomic JSON
report contains requested and achieved rate, counts by type, monotonic duration,
critical bridge sequences, and terminal outcome.

`scripts/event-flood-harness.py` drives that fixture through the production
bridge backend, BitBake supervisor, daemon reducer/journal, Unix IPC, and an
attached client. It records RSS, client frames, resynchronizations, connection
continuity, ordered received sequences, sent/received critical sets, and the
declared journal/snapshot bounds. The retained PERF-IPC audit intentionally
records the former `unbounded_pre_backpressure` terminal starvation. Current
strict mode adds a non-reading client, requires every sentinel at the healthy
client, validates typed pressure counters, and proves a fresh attach after the
flood:

```sh
./scripts/verify-ipc-continuity.sh --event-flood
```

`scripts/test-idle-event-loops.py` runs an isolated daemon with no build
environment, clients, jobs, or PTYs. It samples process CPU and voluntary
context switches for five seconds and bounds shutdown latency. The focused
gate also rejects source regressions to the former one-millisecond
sleep/retry listener, unconditional idle frame rendering, and inactive local
backend polling:

```sh
./scripts/verify-performance.sh --event-loops
```

The client render scheduler is checked independently. Its deterministic tests
record requests, frames, coalesced requests, and idle checks; a 64-update burst
per cadence produces one frame. The source gate requires one centralized,
invalidation-guarded production draw call and a 100 ms minimum normal frame
interval:

```sh
./scripts/verify-performance.sh --render
```

Animation scheduling has its own offline gate. Production uses an explicit 200
ms (5 Hz) animation interval only for visible indeterminate activity and a
separate one-second elapsed-time refresh. The tests reject hidden, determinate,
terminal, overlay-obscured, and reduced-motion animation work:

```sh
./scripts/verify-performance.sh --animations
```

Telemetry scheduling is checked offline as well. The client uses 1 Hz for
visible Dashboard/Tasks metrics and 0.1 Hz elsewhere; background samples do not
redraw. Daemon health pauses without clients, uses 0.2 Hz attached-idle, and 1
Hz with active work. Static source identities are cached, dynamic counters stay
current, histories remain bounded, and source inspection rejects per-sample
process creation:

```sh
./scripts/verify-performance.sh --telemetry
```

Log performance is gated independently. Search normalization occurs once on
ingestion, contiguous daemon records batch within the 64-event/8-ms receive
budget, and one filtered traversal supplies the rendered viewport and position
metadata. The gate retains bounded-buffer, diagnostic-priority, exact-order,
virtualization, and renderer behavior tests:

```sh
./scripts/verify-performance.sh --logs
```

Task-update performance has a separate gate. Contiguous task records batch
within the 64-event/8-ms client budget; only repeated progress for one stable
identity coalesces between lifecycle barriers. Filtered/sorted task identity
order is revision-cached across unrelated frames, live values are not copied,
and active/completed collections remain bounded with explicit overflow counts
and retained terminal failures:

```sh
./scripts/verify-performance.sh --tasks
```

Daemon/client IPC has a measured production-path audit. The pre-optimization
task-event-heavy flamegraph attributed 34.09% self CPU to JSON string escaping,
with full snapshot serialization on the publication path. The optimized
journal keeps a conservative snapshot-size ledger, applies safely bounded
build/log records without cloning the full snapshot, and performs exact full
serialization only when the ledger reaches the protocol limit. Live clients
receive bounded incremental replay rather than a new snapshot whenever they
are more than one service slice behind; identical event frames are serialized
once and reused across attached clients.

The tracked 2,000-event/s production-path observation records the exact release
binary, initial snapshot size, incremental frame min/max/total, frames/s,
bytes/s, daemon CPU time, ordering, and continuity. It requires zero live
snapshot replacements and less than 100 KiB/s for the observed client. It also
retains the honest `unbounded_pre_backpressure` label and expected terminal
starvation: that failure belongs to the next bounded priority-aware ingress and
per-client queue task, so this audit cannot be cited as a backpressure pass.
The retained run measured a 69,029-byte initial snapshot, 312 incremental
events of 99-337 bytes, 74.86 frames/s, 33.58 KiB/s, and 0.21 daemon CPU
seconds over 4.19 seconds.

```sh
./scripts/verify-performance.sh --ipc
```

Backpressure is enforced at both sides of daemon ownership. BitBake events use
separate fixed 512-record reliable and cosmetic lanes; lifecycle, task
transitions, warnings, errors, failures, and disconnects outrank parse/task
progress and ordinary logs. Only the cosmetic lane uses nonblocking loss when
full. A client's pending sequence range is independently bounded by the 4,096
event journal, each service slice advances at most 32 events, and socket reads
are readiness-driven. A full non-reading socket can delay incremental fan-out
by at most two milliseconds before that peer is isolated; bounded handshake
and authoritative snapshot writes retain a one-second saturation allowance.
Healthy peers continue and a fresh client can attach from current authority.

Daemon telemetry exposes `current_queue_depth`, `maximum_queue_depth`,
`cosmetic_coalesced`, `cosmetic_dropped`, `reliable_waits`,
`forced_resynchronizations`, and `slow_client_disconnects`. Nonzero values are
also projected in System Status as `IPC Q current/high C/D/W/R/S`. The strict
4,000-event/s gate requires every warning/error/task/failure/terminal sentinel,
strict client sequence order, zero healthy-client resyncs, one isolated
non-reader, and a successful new attach:

```sh
./scripts/verify-ipc-continuity.sh --backpressure
```

The offline aggregate verifier is `./scripts/verify-performance.sh`.
Steady-state CPU, saturation responsiveness, IPC continuity, and endurance use
`./scripts/verify-low-overhead.sh`,
`./scripts/verify-saturation-responsiveness.sh`,
`./scripts/verify-ipc-continuity.sh`, and
`./scripts/verify-bounded-memory.sh`. Live profiles and real-Poky capture are
explicit evidence roles; the aggregate verifier validates their recorded
identity and freshness without performing network access.
