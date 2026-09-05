# Profiling

`scripts/headless-workload.sh` remains the bounded bridge diagnostic workload.
It isolates configuration and session state in a temporary directory so a
remembered target can never turn diagnostics into a build. The default performs
a bridge handshake and shutdown plus a read-only daemon compatibility
diagnostic; it deliberately does not bypass daemon capability authority to
inspect a workspace. Sanitizer verification selects its optional process-
backend mode.

`scripts/profile-workload.sh` runs the deterministic release workbench benchmark
through the production reducer and Ratatui renderer and writes its frame count,
cell-buffer checksum, and elapsed time to `artifacts/profile/summary.txt`.
`scripts/valgrind.sh` runs 128 frames of the same daemon-independent production
workbench under Memcheck and emits XML plus a human-readable summary under
`artifacts/valgrind/`; it fails on incomplete workload execution,
definite/indirect leaks, unexpected descriptors, or non-runtime findings while
reporting allowlisted Tokio signal descriptors and still-reachable allocations
separately when present.

`scripts/flamegraph.sh` samples the same production workbench benchmark and
writes a validated `artifacts/flamegraph/yoctui.svg` plus its machine-readable
summary. Tooling or host-permission prerequisites fail with actionable exit
status 2. On locked-down Linux hosts, grant `CAP_PERFMON` to `perf` or
temporarily lower `kernel.perf_event_paranoid` according to local security
policy before running the flamegraph gate.

## Next-generation UI rendering matrix

`scripts/test-next-generation-ui-performance.sh` runs five deterministic
160x48 release scenarios with 500 frames each. The default ceiling is
10,000,000 ns/frame and can be lowered explicitly through
`YOCTUI_UI_PERF_MAX_NS_PER_FRAME`; the harness records the UTC timestamp,
frame count, threshold, checksum, elapsed milliseconds, and ns/frame under the
ignored/reproducible `artifacts/profile/next-generation-ui.txt`.

The 2026-08-20 baseline was:

| Scenario | Bounded input | ns/frame |
| --- | --- | ---: |
| Idle | empty Dashboard | 395,995 |
| Active build | 256 tasks, 1,024 logs | 594,910 |
| Large metadata | 4,096 recipes, 1,024 layers | 5,318,065 |
| Log-heavy | 4,096 retained log entries | 845,336 |
| Telemetry | all six histories at the 60-sample bound | 642,610 |

The large-metadata case was the measured outlier, so the fresh flamegraph uses
that scenario rather than the cheaper active-build case. Its validated capture
contains 12,843 perf samples over 6,000 frames, checksum
`f4d850b421930dfd`, and no unresolved/null SVG frames. Fifteen malformed raw
call-chain lines representing 0.3596% of event weight were excluded under the
existing 0.5% quality ceiling. Weighted inclusive events identify
`recipes` (32,687,341,158) well above `layers` (5,657,726,609); inspection of
the symbolized stacks shows that the recipe renderer constructs Ratatui rows
for all 4,096 filtered recipes before the table clips to its viewport. This is
an actionable per-frame allocation/formatting hot path assigned to
`PERF-UI-002`; ordinary Ratatui buffer and Unicode work remains expected.

`PERF-UI-002` bounds recipe and layer row construction to the centered visible
viewport before creating Ratatui `Row`/`Cell` values. The viewport range is a
pure projection of authoritative selection, filtered count, and visible height,
so query/inventory changes cannot leave a retained cache stale. On the same
500-frame matrix, large metadata fell from 5,318,065 to 788,694 ns/frame
(85.2%); idle, active build, log-heavy, and telemetry remained below 0.84
ms/frame. No retained label, query, index, layout, or sparkline cache was
justified after that measured fix.

## M46 pre-optimization runtime profiles

`scripts/capture-runtime-profile.sh` captures named production-process profiles
without rebuilding or changing the measured runtime. The retained M46 profiles
all use the exact pre-optimization v0.1.22 release binary from revision
`11701f9304ddef4ca1ef3926ee4dd7e6a3d7f1f2`, SHA-256
`861da8bda754740e6a7a41675c5fc413223e16f7badce4edb9d9d3ef34ccc0f5`.
Each capture sampled userspace cycles for 15 seconds at 499 Hz with Linux perf
LBR call graphs. Raw `perf.data` is reproducible and deliberately untracked
under `target/performance-profiles/`; validated flamegraphs, symbolized reports,
and machine-readable summaries live under `artifacts/performance/profiles/`.

| Scenario | Authority | Samples | Unresolved weight | Dominant measured work |
| --- | --- | ---: | ---: | --- |
| Idle daemon | Production runtime | 578 | 0.4205% | 1 ms listener retry and empty supervisor polling |
| Idle client | Production runtime | 200 | 0% | Full Ratatui render and terminal-buffer diff at 10 Hz |
| Active build | Real Poky Wrynose 6.0.2 | 653 | 1.0463% | Full snapshot JSON escaping, allocation, and memory movement |
| Log heavy | Deterministic BitBake-like fixture | 4,288 | 0.0853% | Snapshot serialization and repeated retained-log filtering |
| Task-event heavy | Deterministic BitBake-like fixture | 4,303 | 0.2626% | Snapshot serialization, allocation, and task sorting |
| PTY idle | Production daemon PTY | 125 | 0% | Full redraw and terminal-replica reconstruction |
| PTY active | Production daemon PTY at 100 lines/s | 3,504 | 0.1565% | Snapshot serialization and PTY screen serialization |

The real-Poky capture includes sustained `do_compile`, `do_configure`,
`do_install`, `do_package`, and `do_populate_sysroot` execution. Its 1.0463%
unresolved-stack weight is retained explicitly under a declared 1.5%
saturation-only ceiling; all other captures use the normal 0.5% ceiling.
Fixture captures are never presented as real-build evidence.

Reproduce a profile against already-running exact processes with:

```bash
./scripts/capture-runtime-profile.sh \
  --scenario idle-client --duration 15 \
  --revision 11701f9304ddef4ca1ef3926ee4dd7e6a3d7f1f2 \
  --pid daemon=DAEMON_PID --pid client=CLIENT_PID
```

The script records each process identity before sampling, produces the compact
artifacts, and validates symbol resolution. `./scripts/verify-performance.sh
--profiles` then validates all seven authorities, report hashes, sampling
parameters, and required evidence entirely offline.

### Wakeup and timer audit

The matching wakeup audit is retained at
`artifacts/performance/wakeups/manifest.json`. It reuses the exact v0.1.22
10-second-warmup/60-second baseline and adds a 60-second Linux perf software-
counter capture for the daemon plus one idle 160x50 client. The standalone
daemon produced 867.24 voluntary context switches per second. With a client
attached, the baseline split was 39.20/s for the daemon and 19.22/s for the
client; perf independently observed 3,624 aggregate context switches in 60
seconds (60.4/s), 329 CPU migrations, and 1,743.73 ms of task clock.

The source audit identifies the following steady-state periodic work:

- the daemon uses a nonblocking accept with a 1 ms deadline and probes thirteen
  supervisor receivers on every outer-loop pass;
- the client performs a timed IPC receive, every inactive-operation poll,
  global animation tick, and complete Ratatui draw at the default 100 ms UI
  cadence, even when state did not change;
- client host telemetry reads `/proc` network, disk, CPU, memory, and load data
  plus `statvfs` once per second regardless of the visible workspace;
- daemon telemetry scans state and publishes a journal event once per second,
  including when there are no clients;
- reconnect attempts are correctly guarded by disconnection, and PTY screen
  snapshots are output-driven with a 33 ms maximum cadence rather than an idle
  timer;
- an inactive BitBake supervisor has no backend timer, although its empty
  receiver is still polled by the daemon loop;
- Ping/Pong exists in the wire vocabulary, but the production daemon schedules
  no heartbeat.

The host permits perf software counters, but unprivileged scheduling
tracepoints and per-process wakeup-cause counters are unavailable
(`kernel.perf_event_paranoid=4`, scheduler statistics disabled). Ptrace attach
is also blocked. Those absences are explicit in the manifest. A separately
launched strace capture is retained only as qualitative syscall evidence
because tracing perturbs timings; it is not used as CPU gate evidence.

`./scripts/verify-performance.sh --wakeups` validates the source-category
catalog, exact baseline and auxiliary artifact hashes, measurement window,
availability declarations, and observations offline.

## M21 expanded-workbench matrix

`scripts/test-workbench-ux-performance.sh` adds five deterministic 160x48
release scenarios at 300 frames each and retains the same 10,000,000 ns/frame
ceiling. It writes checksummed reproducible output to the ignored
`artifacts/profile/workbench-ux.txt`. The inputs exercise the complete menu,
8,192 rootfs packages plus 8,192 filesystem entries, a normalized 4,096-node
dependency graph, a 4,096-line editor with 1,024 files, and 4,096 retained PTY
rows. The 2026-08-27 measurements were:

| Scenario | ns/frame |
| --- | ---: |
| Menu-heavy | 789,382 |
| Large rootfs | 1,392,321 |
| Large dependency graph | 5,009,888 |
| Large editor | 2,334,390 |
| Dense terminal | 1,044,688 |

The same run left the original five 500-frame scenarios below 929,596
ns/frame. No cache was added: viewport projection and existing model bounds
were sufficient. `scripts/test-flamegraph.sh` reuses the isolated release
target and its validator fixture confirms 1,000 resolved samples and zero
unresolved frames.
