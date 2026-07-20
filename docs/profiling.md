# Profiling

`scripts/headless-workload.sh` is the deterministic bridge workload used by every profiling script. It performs a protocol handshake, workspace inspection, typed metadata queries, and clean bridge shutdown without needing a real Yocto build.

`scripts/profile-workload.sh` writes a release workload timing summary to `artifacts/profile/summary.txt`. `scripts/valgrind.sh` emits XML and a human-readable summary under `artifacts/valgrind/`, failing on Valgrind errors. `scripts/flamegraph.sh` writes `artifacts/flamegraph/yoctui.svg` when `cargo-flamegraph` is installed. Tooling prerequisites fail with actionable exit status 2.
