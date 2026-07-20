# Profiling baseline

Date: 2026-07-20

The deterministic release bridge workload completed in **0.142 seconds** wall-clock time on the development host. The workload performs bridge startup, protocol negotiation, workspace inspection, typed metadata queries, and clean shutdown; it does not contact a live BitBake server.

Reproduce with:

```sh
./scripts/profile-workload.sh
```

Timing is environment-dependent and is recorded only as a regression baseline. The generated raw timing output is ignored at `artifacts/profile/summary.txt`.

## Valgrind baseline

The same debug bridge workload was run with Memcheck on 2026-07-20. It reported 0 bytes definitely lost, 0 indirectly lost, and 0 possibly lost. It retained 59,872 bytes reachable at process shutdown and reported two Tokio runtime signal-registry file descriptors. These non-fatal runtime resources are reported in the generated XML; the script fails on definite or indirect leaks and any non-runtime Memcheck error.
