# Profiling

`scripts/headless-workload.sh` is the deterministic bridge workload used by every profiling script. It isolates configuration and session state in a temporary directory so a remembered target can never turn profiling into a build. The default performs a protocol handshake, workspace inspection, typed metadata queries, and clean bridge shutdown without needing a real Yocto build; sanitizer verification selects its optional process-backend mode.

`scripts/profile-workload.sh` writes a release workload timing summary to `artifacts/profile/summary.txt`. `scripts/valgrind.sh` emits XML and a human-readable summary under `artifacts/valgrind/`; it fails on definite/indirect leaks or non-runtime Memcheck findings, while reporting Tokio shutdown descriptors and still-reachable allocations separately. `scripts/flamegraph.sh` writes `artifacts/flamegraph/yoctui.svg` when `cargo-flamegraph` is installed. Tooling prerequisites fail with actionable exit status 2.
