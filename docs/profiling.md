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
`scripts/valgrind.sh` uses the native process backend so Memcheck observes only
Rust-owned execution and emits XML plus a human-readable summary under
`artifacts/valgrind/`; it fails on definite/indirect leaks, unexpected
descriptors, or non-runtime findings while reporting the two Tokio signal
descriptors and still-reachable allocations separately.

`scripts/flamegraph.sh` samples the same production workbench benchmark and
writes a validated `artifacts/flamegraph/yoctui.svg` plus its machine-readable
summary. Tooling or host-permission prerequisites fail with actionable exit
status 2. On locked-down Linux hosts, grant `CAP_PERFMON` to `perf` or
temporarily lower `kernel.perf_event_paranoid` according to local security
policy before running the flamegraph gate.
