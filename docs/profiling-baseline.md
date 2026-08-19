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

## Valgrind baseline

The 128-frame debug production workbench was run with Memcheck on 2026-08-19.
It reported 0 bytes definitely lost, 0 indirectly lost, 0 possibly lost, 544
bytes still reachable, and no open descriptors. These non-fatal reachable
allocations remain reported in the generated XML; the script fails on definite
or indirect leaks, unexpected descriptors, incomplete workload execution, and
any non-runtime Memcheck error.
