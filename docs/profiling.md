# Profiling

`scripts/profile-workload.sh` runs the deterministic headless process fixture. `scripts/valgrind.sh` emits XML and a summary, failing on real Valgrind errors. `scripts/flamegraph.sh` records a release workload when `cargo flamegraph` is installed. Default workloads never need a real Yocto build.
