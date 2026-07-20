# Profiling baseline

Date: 2026-07-20

The deterministic release bridge workload completed in **0.142 seconds** wall-clock time on the development host. The workload performs bridge startup, protocol negotiation, workspace inspection, typed metadata queries, and clean shutdown; it does not contact a live BitBake server.

Reproduce with:

```sh
./scripts/profile-workload.sh
```

Timing is environment-dependent and is recorded only as a regression baseline. The generated raw timing output is ignored at `artifacts/profile/summary.txt`.
