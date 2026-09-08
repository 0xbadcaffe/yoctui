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
force a redraw. Idle clock updates are at most 1 Hz. Visible ordinary activity
animation and full-frame presentation run at 4 Hz below contention. During a
live build, a measured host utilization of at least 90% switches cosmetic
presentation to 1 Hz; input and resize remain immediate and independent of
that cadence. This adaptive bound is required by the one-logical-CPU budget:
the real-Poky measurement showed that 4 Hz full 160x50 reconstruction alone
could exceed the complete process budget under saturation. Active PTY screen
publication remains at most 30 frames/s. Reduced-motion mode freezes animation.
Hidden animation and telemetry cannot invalidate the frame.

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

The refreshed v0.1.51 release observation retains 1,800 one-second samples after a
10-second warmup while the production bridge/daemon/IPC path receives 4,000
events/s. Daemon RSS grew 1,318,912 bytes and attached-observer RSS grew 262,144
bytes; least-squares slopes over the final 20 minutes were 0 and 1,142
bytes/minute respectively.
Threads stayed at three and one respectively. Critical retention, strict event
order, and connection continuity remained true. The harness bounds its own
sequence and RSS observations, and the default verifier separately reruns the
one-minute fixture plus model/protocol retention tests.

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

### Supported real-Poky saturation evidence

The [fresh v0.1.89 observation](../artifacts/performance/real-poky/v89-linux-yocto-do-compile.json)
passes the unchanged real-Poky validator with 360 unprofiled samples after a
ten-second warmup. Combined CPU is **0.5831064335% of one logical CPU**, down
from the failed v88 observation at 1.0334662486%. Daemon/client independently
trimmed CPU is 0.1788248486%/0.3959064743%. Host utilization is 99.7892% of
capacity and BitBake tree CPU is 308.6674% of one logical CPU. Input-to-frame
p95 is 4.558337 ms; build/cancel acknowledgement is 0.676482/6.060129 ms,
fresh attachment 50.053626 ms. Queue maximum is 90/256, rendering 1.289 fps;
there are no drops, reliable waits, resynchronizations or backend disconnects.
Reconnect and owned-job cancellation passed, and all capture processes exited.
No Cargo or profiler ran during either acceptance window.

The exact release binary is preserved at
`/home/bspguy-dev/.local/state/yoctui-v89-release.N94Frh/yoctui`, SHA-256
`261fb7a45026d7a869c9ae0f8804b7c813c226d2e3d62238fad76b0c795908fd`.
The [canonical manifest](../artifacts/performance/real-poky/manifest.json)
binds the actual record, all runtime Rust/bundled bridge source hashes, and
the [v89 source patch](../artifacts/performance/real-poky/v89-source.patch)
to d2214e8. Reapply zero-context patches with `git apply --unidiff-zero`.
Fresh [idle evidence](../artifacts/performance/results/low-overhead-v89/measurement.json)
also passes unchanged limits: daemon 0.0416%, client 0.1248%, combined 0.2704%
of one CPU. These actual records replace the canonical v64 real/idle results;
historical records remain in Git and the failed v88 record remains separately
preserved. The regenerated regression record passes 22 hard metrics and seven
correctness checks. The complete `--real-poky-evidence` verification chain also
passes, including production-path, saturation, scheduling and coexistence
checks. The global completion gate remains a separate requirement, not a claim
inferred from these measurements.

RELEASE-PERF-REFRESH-001 tracks source-bound evidence after the OpenBMC repairs;
historical v0.1.64 evidence cannot certify current sources. The isolated fixture is
`/home/bspguy-dev/.local/state/yoctui-release-poky.xWaZbs`, initialized from the
existing supported Poky checkout. Read-only `bitbake -e linux-yocto` confirms
MACHINE `qemux86-64`, DISTRO `poky`, BitBake 2.18.0 and kernel 6.18.24+git.
TOPDIR, TMPDIR, WORKDIR, DL_DIR, SSTATE_DIR and the sstate cleanup path pattern
all resolve inside this new fixture. Existing source/sstate caches are read
mirrors only; separate shared bare Git repositories read the old object pools.
The normal build and the old `/tmp/yoctui-m52-poky.nHLLjU` fixture are preserved.
Offline mode, eight build/make workers, STOPTASKS at 15 GiB and HALT at 8 GiB
are configured only in the new fixture. The initial v0.1.88 release build passed;
the exact preserved binary has SHA-256
`be5fa7261cd3ebaf7f6f1786eabb46736419d043086d0e92224f3cc63f48b510`.

The [fresh v88 capture](../artifacts/performance/real-poky/v88-linux-yocto-do-compile.json)
completed 360 samples after ten seconds of warmup on September 8, 2026, but
**failed** the unchanged combined CPU gate: 1.0334662486% > 1.00% of one
logical CPU. Daemon and client independently trimmed means are 0.5872427701%
and 0.4039586047%; adding those trimmed means is not the combined metric,
which trims the per-sample sums. Host utilization was 99.791% of capacity and
the BitBake tree used 304.048% of one logical CPU. Input-to-frame p95 was
4.595 ms; build/cancel acknowledgements were 3.619/3.608 ms, fresh attachment
40.886 ms. Queue maximum was 87/256 with no drops, resynchronization or backend
disconnect; reconnect and owned-job cancellation passed. No Cargo or profiler
ran during this measurement. The capture completed its own process cleanup.
The [failed-candidate manifest](../artifacts/performance/real-poky/v88-failed-manifest.json)
and [source patch](../artifacts/performance/real-poky/v88-source.patch) preserve
exact provenance without being promoted as passing canonical evidence.
RELEASE-DAEMON-CPU-001 was registered before runtime edits. Profiles are
diagnostic only; acceptance requires a new unprofiled release observation.
The old fixture used tmpfs and the new one
uses ext4, so this comparison alone does not establish a code regression.

The separately labeled [v88 diagnostic profile](../artifacts/performance/profiles/v88-diagnostic-notes.md)
contains 275 actual userspace-cycle samples with no lost samples or unresolved
stack weight. Full snapshot serialization accounts for 26.90% inclusive cycle
weight; journal publication totals 43.81%. Filtering the raw flat report to
the daemon retains all 19.23% self-weight JSON string escaping. Source tracing
finds once-per-second telemetry cloning and serializing unchanged workspace
metadata despite not retaining telemetry in the snapshot. The v89 candidate
extends the existing conservative size-ledger fast path to telemetry only,
retaining all bounds, exact remeasurement and transactional rejection. Two
failed-first tests reproduce 101 serializations for 100 events and failure to
amortize constrained headroom; four focused regressions additionally cover
ordered wire replay, byte-limit rejection and exhausted sequence/generation.
Fresh unprofiled v89 acceptance is recorded above; the profiled workload is
never substituted for CPU evidence. Its conversion exceeded the external controller
timeout after recording, completed afterward, and passed the unchanged
summarizer's sample/quality checks separately. Diagnostic processes all exited.

The first read-only capture-startup probe (preserved v87 production code,
all daemon commands prohibited) exposed a harness assumption: attachment can
precede asynchronous initial workspace publication. The capture now waits for
actual workspace variables with a 300-second deadline and 8192-message bound,
responds to heartbeat pings, rejects changed instance snapshots, and surfaces
metadata failure, missing variables, EOF and timeout. Three failed-first
regression tests cover these cases and run in the real-Poky verifier. This
readiness work occurs before any build command or measurement; warmup, workload
trigger, sampling and all performance/source-validation thresholds are unchanged.
The [actual read-only recheck](../artifacts/performance/real-poky/v88-readonly-startup-probe.txt)
reached its intentional stop after metadata readiness and before any command.
It is functional evidence only, not a release CPU or latency measurement.

The historical v0.1.51 observation uses Poky 6.0.2, `qemux86-64`, distro
`poky`, `BB_NUMBER_THREADS=8`, and `PARALLEL_MAKE=-j 8`. Through the isolated
production daemon it runs `linux-yocto:do_cleansstate`, starts
`linux-yocto:do_compile`, waits for that exact task-start event, attaches one
real 160x50 interactive client, gathers 100 input-to-visible-frame probes,
warms for 10 seconds, and then records 119 one-second samples across a
120-second sustained compile window. The key probes precede the CPU warmup, so
they demonstrate saturated responsiveness without redefining steady-state
normal operation.

The v0.1.51 measured binary
(`a63bee5996f134b98de9ab243ab22b8cb5b5aff4a66bc4d13f8a4c36bacb18b0`)
held the host at a 99.6836% trimmed-mean utilization. Captured BitBake/server,
worker, and descendant compiler CPU was 301.6194% of one logical CPU. Yoctui
used 0.4297% daemon CPU and 0.4496% client CPU; the independently calculated
combined trimmed mean was **0.9207% of one logical CPU**. Input-to-frame p95
was 5.4191 ms, build/cancellation acknowledgement was 0.8045/0.3061 ms, and a
fresh attach took 35.0212 ms. Queue depth peaked at 31 of 256, with zero
backend disconnects, forced resynchronizations, reliable waits, or cosmetic
drops. Cancellation was acknowledged and accepted after the full window.

The exact raw samples, host identity, repository revisions and dirty diff
hash, process start identities, binary identity, pressure counters, and source
hashes are retained in Git history under `artifacts/performance/real-poky/`.
The historical v0.1.64 observation instead contains 360 samples and 0.9333%
combined CPU, with 4.6814 ms input p95 and 4.8349 ms cancellation acknowledgement.
Neither historical version certifies the current runtime; fresh v89 acceptance
and the failed v88 observation above remain explicit. The verifier
rejects fixture roles, a non-kernel trigger, insufficient host/BitBake load,
or any value outside the CPU, latency, rendering, queue, cancellation, and
continuity contract.

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

### OS scheduling policy

The scheduler audit uses `scripts/scheduler-latency-probe.py` as a low-duty
interactive proxy: it requests a wake every 10 ms using `CLOCK_MONOTONIC` and
records p50/p95/p99/maximum lateness, actual nice level, cgroup, CPUWeight, and
CPU time. `scripts/measure-scheduling.py` runs three three-second trials for
each policy while the deterministic load fixture keeps all eight reference-host
CPUs runnable. Hashed evidence is retained under
`artifacts/performance/scheduling/`.

The inherited nice-0 trials had 0.0781 ms median p95 wake latency. Deliberately
deprioritizing the probe to nice 5 increased that median to 0.4696 ms. Requesting
`nice -n -5` without privilege printed a permission warning and the child still
ran at nice 0, so negative nice is neither portable nor recommended. A transient
systemd user service successfully applied `CPUWeight=200`, but its 0.0871 ms
median p95 did not materially improve the inherited result. Every trial used all
eight affinity CPUs, reached at least 74% CPU on its least-loaded worker, reaped
all children, and kept p95 far below the 100 ms responsiveness contract.

The release policy is therefore to inherit the user's normal scheduling policy.
Yoctui does not change nice, install a CPUWeight override, request real-time
scheduling, or require root. Administrators may independently assign cgroup
weights as a host policy, but the reference evidence does not justify making
that a Yoctui recommendation or default. Reproduce the evidence offline with:

```sh
./scripts/measure-scheduling.py \
  --revision "$(git rev-parse HEAD)" \
  --duration-seconds 3 \
  --repetitions 3 \
  --output /tmp/yoctui-scheduling.json
./scripts/verify-performance.sh --scheduling
```

The gate treats absence of a systemd user manager as supported; correctness and
the required latency check always use inherited priority.

### CPU affinity and isolation

`scripts/measure-affinity.py` repeats the same 10 ms monotonic wake probe in
three explicit CPU layouts: both load and probe use the full affinity set; the
probe is pinned to a logical CPU which still has a load worker; and the probe is
pinned to one logical CPU omitted from the load. The reference host has eight
logical CPUs with SMT sibling pairs; selected CPU 7 shares a physical core with
CPU 3. Each layout has three three-second trials and records the exact load and
probe sets, sibling topology, load floor, cleanup, and latency tails.

The required inherited/full-set path measured 0.0835 ms median p95. Pinning to
the competing CPU measured 0.5791 ms. Omitting CPU 7 from load but continuing to
load its sibling CPU 3 measured 1.9253 ms. All remain far below 100 ms, but the
nominal logical reservation was materially worse because it removed migration
without isolating the physical core. This evidence does not justify automatic
or recommended affinity. Yoctui inherits the caller's affinity and never
hardcodes a CPU. An administrator who understands the machine topology may
still launch both BitBake and Yoctui in explicit cpusets, but correctness and
support do not depend on it.

```sh
./scripts/measure-affinity.py \
  --revision "$(git rev-parse HEAD)" \
  --duration-seconds 3 \
  --repetitions 3 \
  --output /tmp/yoctui-affinity.json
./scripts/verify-performance.sh --affinity
```

The gate also reruns the standard saturation harness with no reserved CPU.

### BitBake coexistence guidance

The read-only `yoctui inspect` diagnostic reports the current logical-CPU
count, one-minute load average, `BB_NUMBER_THREADS`, and the numeric job limit
from common `PARALLEL_MAKE` forms. It classifies sustained load above 1.5
runnable tasks per logical CPU, or either configured limit above twice the CPU
count, as oversubscribed. A load or configured limit near the CPU count is
reported as busy. These are review signals, not proof of a bad configuration:
BitBake task concurrency and each task's make concurrency overlap dynamically,
so Yoctui deliberately does not multiply the two values.

The reference audit used every one of eight affinity CPUs at about 99% host CPU
and repeated three three-second trials. One deterministic worker per CPU
measured 0.6223 ms median p95 scheduler wake latency; two workers per CPU
measured 1.0377 ms. The latter models runnable oversubscription, not BitBake's
exact workload. The earlier scheduling audit found no benefit from a transient
`CPUWeight=200` user service, while the affinity audit found that pinning and a
nominal logical-CPU reservation were worse on this SMT host. Together these
measurements retain inherited scheduling, cgroup, and affinity as the required
path.

If a real build is oversubscribed and interaction is poor, the diagnostic may
show `available logical CPUs - 1` as a review-only example for both
`BB_NUMBER_THREADS` and the `PARALLEL_MAKE` job count. This is not an automatic
default and may reduce build throughput. Yoctui never edits `local.conf`,
environment variables, or BitBake policy. It remains correct without root,
cgroup changes, CPU isolation, or a deliberately free CPU. Administrators may
still tune BitBake or cgroups after reviewing workload-specific evidence.

```sh
yoctui --build-dir "$BUILDDIR" inspect
./scripts/measure-bitbake-coexistence.py \
  --revision "$(git rev-parse HEAD)" \
  --duration-seconds 3 \
  --repetitions 3 \
  --output /tmp/yoctui-coexistence.json
./scripts/verify-performance.sh --coexistence
```

### Steady-state release CPU gate

The release CPU gate starts an isolated daemon without a build environment,
measures it alone, then attaches one real interactive client through a fixed
160x50 PTY whose output is continuously drained. Startup is bounded at five
seconds and excluded. Each scenario then excludes a ten-second warmup and
records sixty one-second `/proc/PID/stat` field 14+15 deltas. CPU is reported in
normal process-accounting units as a 10% trimmed mean of one logical CPU; it is
never divided by the host CPU count.

The first attached-client probe exposed a one-millisecond daemon service poll:
an otherwise idle connection caused about 863 voluntary wakeups/s and 6.25%
daemon CPU. The daemon now makes one kernel readiness wait over its listener and
all current client sockets. An idle attached client therefore uses the 100 ms
idle bound while input or a new connection wakes service immediately; active
jobs retain a 35 ms supervisor-service bound. BitBake publication also writes
one coalesced byte to a nonblocking process-local readiness pair. The same
kernel wait therefore wakes immediately for backend work, and draining the
notification rearms it without one wakeup per queued event. Failures,
disconnects, cancellation, and terminal outcomes signal immediately. Retained
nonterminal transitions plus cosmetic log/progress publication coalesce
readiness to at most once per 30 ms so a fast producer cannot overrun a healthy
decoder; batching never changes retention or order.

The v0.1.51 release result records 0.0000% idle-daemon, 0.0208% idle-client,
and 0.1041% combined trimmed-mean CPU of one logical CPU. The attached daemon
itself recorded 0.0000%. These independently pass the 0.20%, 0.50%, and 1.00%
limits. The retained records include all sixty raw samples, process start
identities, host and filesystem identity, exact binary and source hashes,
startup times, terminal geometry, and drained PTY output. The offline
validator recalculates every trimmed mean and the default gate repeats the full
release measurement without network access:

```sh
./scripts/verify-low-overhead.sh
```

### Saturated input latency

The release input probe runs in a real fixed 160x50 PTY, enables Crossterm mouse
capture, and reads terminal events through `event::read`. Keyboard navigation
uses the production focus route; mouse scrolling uses the production
coordinate-aware route. Both apply the resulting typed action through the model
reducer and draw with the production UI renderer. The probe emits an OSC marker
only after the terminal draw returns. The driver timestamps each sequential PTY
write and the probe records event receipt, reducer completion, and frame
completion from the shared Linux `CLOCK_MONOTONIC` clock.

After one second of CPU-load warmup, the reference run collected 100 distinct
visible selection changes for each input kind while one pinned load worker ran
on every one of eight affinity CPUs. Host CPU reached 99.75%, and the least-busy
load worker reached 80.70%. Keyboard-to-model latency measured 0.965 ms p50 and
1.974 ms p95; keyboard-to-visible-frame measured 2.270 ms p50 and 4.336 ms p95;
mouse-to-visible-selection measured 2.270 ms p50 and 4.440 ms p95. All three are
well below the 100 ms contract. Raw timestamps, exact binary identity, host,
terminal, and load evidence live under `artifacts/performance/input-latency/`.

```sh
./scripts/verify-saturation-responsiveness.sh --input-latency
```

### Saturated IPC latency

The release IPC probe uses the production bridge backend, bounded BitBake
supervisor, daemon snapshot journal, shared event fanout, and AF_UNIX
length-prefixed JSON transport. A dedicated offline bridge embeds Linux
`CLOCK_MONOTONIC` timestamps and stable identities in ordinary log events; the
client receipt therefore conservatively includes bridge-to-daemon ingestion as
well as daemon event delivery. A correlated `not_found` result for a
deliberately absent job bounds command receipt without unrelated command work.
Two protocol-compliant batches of cancellation requests target live daemon
BitBake jobs, with every correlated acknowledgement and outcome retained.

After one second of warmup, the refreshed reference run collected 100
observations per path while one pinned worker ran on every one of eight affinity
CPUs. Host CPU reached 99.26%, and the least-busy worker reached 78.99%.
Daemon-event delivery measured 2.343 ms p50 and 3.262 ms p95; command receipt
measured 0.084 ms p50 and 0.229 ms p95; cancellation acknowledgement measured
1.118 ms p50 and 1.359 ms p95. All 100 cancellation requests were acknowledged
and accepted. Protocol sequences were strictly increasing, no backend
disconnect was observed, explicit detach was acknowledged in 0.119 ms, and a
new client attached before saturation ended. Raw timestamps and exact
binary/source identity live under `artifacts/performance/ipc-latency/`.

```sh
./scripts/verify-ipc-continuity.sh --latency
```

`scripts/fixtures/bitbake-event-flood-bridge.py` is the deterministic bridge
fixture. It accepts a rate, duration, balanced/log-heavy/task-heavy profile,
and success/failure/disconnect terminal mode. Stable task identities plus
ordered critical sentinels cover ordinary logs, progress, warnings, errors,
task failures, cancellation, backend EOF, and build terminals. Its atomic JSON
report contains requested and achieved rate, counts by type, monotonic duration,
critical bridge sequences, and terminal outcome. Progress is explicitly
coalescible; warning, error, queued/started lifecycle, failure, cancellation,
disconnect, and build-terminal evidence remains mandatory.

`scripts/event-flood-harness.py` drives that fixture through the production
bridge backend, BitBake supervisor, daemon reducer/journal, Unix IPC, and an
attached client. It records RSS, client frames, resynchronizations, connection
continuity, ordered received sequences, sent/received critical sets, and the
declared journal/snapshot bounds. The retained PERF-IPC audit intentionally
records the former `unbounded_pre_backpressure` terminal starvation. Current
strict mode adds a non-reading client, requires every correctness sentinel at the healthy
client, validates typed pressure counters, and proves a fresh attach after the
flood:

```sh
./scripts/verify-ipc-continuity.sh --event-flood
```

`scripts/test-idle-event-loops.py` runs an isolated daemon with no build
environment, clients, jobs, or PTYs. It samples process CPU and voluntary
context switches for ten seconds and bounds shutdown latency. The focused
gate also rejects source regressions to the former one-millisecond
sleep/retry listener, unconditional idle frame rendering, and inactive local
backend polling:

```sh
./scripts/verify-performance.sh --event-loops
```

The client render scheduler is checked independently. Its deterministic tests
record requests, frames, coalesced requests, and idle checks; a 64-update burst
per cadence produces one frame. The source gate requires one centralized,
invalidation-guarded production draw call, a 250 ms normal frame interval, and
the 1 Hz saturated-build presentation interval:

```sh
./scripts/verify-performance.sh --render
```

Animation scheduling has its own offline gate. Production uses an explicit 250
ms (4 Hz) animation interval only for visible indeterminate activity, slows
cosmetic animation to 1 Hz during a saturated live build, and uses a separate
one-second elapsed-time refresh. The tests reject hidden, determinate, terminal,
overlay-obscured, and reduced-motion animation work:

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
build/log/job/telemetry records without cloning the full snapshot, and performs exact full
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

## Regression record

`artifacts/performance/regression/metrics.json` is the compact, deterministic
comparison surface for M46. It collects idle and real-Poky CPU, saturated input
and IPC latency, idle context-switch rates, saturated render cadence, event-flood
and live-build pressure, plus the 30-minute memory result. Every input path and
SHA-256 digest is embedded in the record, and the manifest additionally binds
the generator and its tests.

Controlled release metrics keep their exact hard limits: in particular, the
combined daemon/client CPU ceiling remains 1% of one logical CPU. Correctness
booleans for critical retention, event order, continuity, cancellation, and
reconnect are also hard failures. Measurements without a controlled comparison
environment, such as informational wakeup and absolute RSS trends, are recorded
but do not fail for tiny variance. A comparison is meaningful only when its
scenario and method identity match.

The offline verifier validates every source digest, regenerates the record
byte-for-byte from the retained evidence, and then checks all hard metrics and
correctness outcomes:

```sh
./scripts/verify-performance.sh --regressions
```

## Continuous integration tiers

`scripts/verify-performance-ci-fast.sh` is the bounded push and pull-request
gate. It checks idle blocking, dirty rendering, task/log/job coalescing,
full-affinity saturation, and production IPC backpressure. It intentionally
does not run a full image build or a 30-minute endurance sample.
The flood assertion follows the priority model: progress is coalescible, but
failure, terminal, warning, error, and task-lifecycle sentinels are mandatory.

Weekly and manually dispatched CI builds the release binary, validates the
retained flamegraph and real-Poky roles, repeats the 60-second low-overhead
measurement, and captures a fresh 30-minute memory result. Fresh real-Poky
capture is separately opt-in through `YOCTUI_LIVE_PERFORMANCE=1` on a labeled
self-hosted runner with `YOCTUI_PERF_BUILD_DIR` and `YOCTUI_PERF_POKY_ROOT`.
Pull requests therefore cannot accidentally download or modify a Poky tree.
Each performance job uploads `artifacts/performance/ci/` under `if: always()`.

The complete offline CI contract and fast execution path are verified with:

```sh
./scripts/verify-performance.sh --ci
```

## Operator and developer quick reference

| Question | Implemented behavior | Verification |
| --- | --- | --- |
| How much CPU? | idle daemon <=0.20%, idle client <=0.50%, daemon plus one client <=1.00% of one logical CPU | `./scripts/verify-low-overhead.sh` |
| Does input remain responsive? | key/mouse-to-visible-frame p95 <=100 ms with every affinity CPU runnable | `./scripts/verify-saturation-responsiveness.sh` |
| Can a slow client stall BitBake? | no; each client is isolated by bounded replay and write deadlines | `./scripts/verify-ipc-continuity.sh` |
| Are buffers bounded? | logs, tasks, journal, IPC queues, telemetry, and PTY scrollback have hard caps | `./scripts/verify-bounded-memory.sh` |
| Does real BitBake remain connected? | real EOF is authoritative; scheduling delay alone is not disconnect | `./scripts/verify-performance.sh --real-poky-evidence` |
| How are frames scheduled? | event/dirty driven, ordinary 4 Hz, saturated cosmetic 1 Hz, input immediate | `./scripts/verify-performance.sh --render` |
| How is telemetry sampled? | visible 1 Hz, background 0.1 Hz, daemon idle 0.2 Hz, no-client paused | `./scripts/verify-performance.sh --telemetry` |

For a new CPU profile, build with release symbols, start the exact workload,
then pass every measured process as `ROLE=PID`:

```sh
cargo build --release --locked -p yoctui --all-features
./scripts/capture-runtime-profile.sh \
  --scenario idle-client \
  --duration 20 \
  --binary target/release/yoctui \
  --revision "$(git rev-parse HEAD)" \
  --pid daemon=DAEMON_PID \
  --pid client=CLIENT_PID
./scripts/verify-performance.sh --profiles
```

Capture a fresh supported real-Poky sample only from an initialized build
environment. This command cleans only `linux-yocto` sstate, waits for its real
`do_compile` start, measures 120 sustained seconds, and cancels that owned job
afterward:

```sh
source /path/to/poky/oe-init-build-env /path/to/build
cd /path/to/yoctui
./scripts/capture-real-poky-performance.py \
  --binary target/release/yoctui \
  --build-dir "$BUILDDIR" \
  --poky-root /path/to/poky \
  --target linux-yocto \
  --task compile \
  --preclean \
  --wait-for-task linux-yocto:do_compile \
  --warmup-seconds 10 \
  --seconds 120 \
  --output /tmp/yoctui-real-poky.json
```

No supported path requires root, real-time scheduling, a reserved CPU, a nice
change, or a cgroup override. `yoctui inspect` may suggest reviewing
`BB_NUMBER_THREADS` and `PARALLEL_MAKE` on an oversubscribed host, but Yoctui
never changes them. Optional `taskset`, cpuset, nice, or systemd CPUWeight
experiments remain administrator policy and must be measured on the actual
machine; the reference evidence found no benefit over inherited defaults.

Steady-state CPU, saturation responsiveness, IPC continuity, and endurance use
`./scripts/verify-low-overhead.sh`,
`./scripts/verify-saturation-responsiveness.sh`,
`./scripts/verify-ipc-continuity.sh`, and
`./scripts/verify-bounded-memory.sh`. Live profiles and real-Poky capture are
explicit evidence roles; the aggregate verifier validates their recorded
identity and freshness without performing network access.
