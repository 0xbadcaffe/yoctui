# Profiling baseline

Date: 2026-07-20

The former deterministic release bridge workload completed in **0.142 seconds** wall-clock time on the development host. That historical workload performed bridge startup, protocol negotiation, workspace inspection, typed metadata queries, and clean shutdown without contacting a live BitBake server. It predates daemon-owned compatibility authorization and is retained only as historical timing evidence.

The current workload renders 6,000 deterministic 160x48 frames through the
production reducer and Ratatui UI without requiring daemon or BitBake authority.
The 2026-08-19 optimized baseline completed in **3.731 seconds** with cell-buffer
checksum `95f340a128cd6012`. This is intentionally not compared directly with
the historical bridge-startup observation because it measures sustained CPU
rendering rather than protocol lifecycle latency.

Reproduce with:

```sh
./scripts/profile-workload.sh
```

Timing is environment-dependent and is recorded only as a regression baseline.
The generated timing output is ignored at `artifacts/profile/summary.txt`.

## M46 pre-optimization process baseline

The 2026-09-05 baseline uses release v0.1.22 at revision `11701f93`, a
160x50 PTY, a 10-second warmup, and sixty one-second `/proc` samples. CPU is a
10% trimmed mean in percent of one logical CPU. The host had eight logical
Intel i7-8650U CPUs, Linux 7.0.0-30-generic, 14.5 GiB RAM, and an ext4 build
filesystem. The complete raw samples and byte hashes are tracked in
`artifacts/performance/baseline/manifest.json`.

| Scenario | Daemon CPU | Client CPU | Combined CPU | Max RSS daemon/client |
| --- | ---: | ---: | ---: | ---: |
| Idle daemon | 5.42% | — | 5.42% | 10.8 MiB / — |
| Idle attached | 0.19% | 2.21% | 2.43% | 10.9 / 16.3 MiB |
| Real Wrynose build | 15.16% | 6.14% | 21.62% | 25.6 / 23.7 MiB |
| PTY attached, idle | 1.06% | 2.29% | 3.39% | 20.3 / 19.1 MiB |
| PTY, 100 lines/s | 0.90% | 2.29% | 3.21% | 21.1 / 20.5 MiB |
| 2,000-event/s fixture | 44.34% | 11.18% | 55.36% | 83.3 / 21.0 MiB |
| Two idle clients | 0.08% | 2.00% + 2.08% | 4.22% | 11.0 / 16.1 MiB each |

The idle daemon produced 867 voluntary context switches/s because its empty
listener used a 1 ms retry. Each client invoked rendering at 10 Hz even when
only the clock or no visible state changed. The real Wrynose 6.0.2
`core-image-full-cmdline` sample ran sustained compile tasks on all eight CPUs.
The deterministic flood bridge requested 2,000 events/s through the production
bridge/supervisor/journal/IPC path. Its bridge completed, but the unbounded
supervisor channel retained roughly 180,000 events behind a 32-event outer-loop
drain; the terminal build outcome remained starved and the client repeatedly
required snapshot resynchronization. These are failing baseline observations,
not release claims.

The pre-instrumentation protocol did not expose exact per-client IPC byte or
message counters. That absence is recorded rather than estimated in the real
build artifact; the fixture rate and 10 Hz render count are directly controlled
or counted. Wakeup reasons require `perf` scheduling samples and advance under
PERF-WAKEUPS-001. Reproduce process accounting with:

```sh
./scripts/measure-process-overhead.py --help
./scripts/verify-performance.sh --baseline
```

## Valgrind baseline

The 128-frame debug production workbench was run with Memcheck on 2026-08-19.
It reported 0 bytes definitely lost, 0 indirectly lost, 0 possibly lost, 544
bytes still reachable, and no open descriptors. These non-fatal reachable
allocations remain reported in the generated XML; the script fails on definite
or indirect leaks, unexpected descriptors, incomplete workload execution, and
any non-runtime Memcheck error.
