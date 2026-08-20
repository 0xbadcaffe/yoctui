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
