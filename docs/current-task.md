# Current Task

## Task

**ID:** HARDEN-ANALYSIS-001
**Title:** Integrate hardening analysis gates
**Status:** BLOCKED

## Objective

Complete the deterministic Flamegraph evidence and close the final hardening
analysis task without skipping a required tool or weakening the completion
gate.

## Completed evidence

- Linux pseudo-terminal restoration passes.
- Property and repeated process-tree stress passes.
- ASan and LSan pass their isolated workloads.
- Valgrind reports no definite, indirect, or possible lost bytes; the two
  Tokio signal descriptors are explicitly recognized.
- Deterministic release profiling produces a nonempty summary.
- `cargo-flamegraph 0.6.13` and matching `perf 7.0.12` are installed.

## External blocker

The host currently reports:

```sh
cat /proc/sys/kernel/perf_event_paranoid
# 4
```

It denies even the userspace dummy probe:

```sh
perf record --no-buildid-mmap -e dummy:u -o /tmp/yoctui-perf.data -- true
```

`./scripts/flamegraph.sh` therefore exits 2 with `perf sampling is
unavailable`. Repository code cannot grant host perf capability.

## Required external action

Under the local host security policy, grant `CAP_PERFMON` to the matching perf
binary or temporarily lower the sysctl, for example:

```sh
sudo sysctl -w kernel.perf_event_paranoid=0
```

Then continue immediately with the verification below.

## Verification

```bash
./scripts/test-terminal.sh
./scripts/valgrind.sh
./scripts/profile-workload.sh
./scripts/flamegraph.sh
test -s artifacts/flamegraph/yoctui.svg
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

## Definition of done

- The Flamegraph command succeeds with a nonempty deterministic SVG.
- `HARDEN-ANALYSIS-001` and then `HARDEN-001` are marked `DONE` only after
  every required command passes.
- The strict completion gate passes without skipped tooling or live-support
  overclaims.

## Next task

No other eligible required task remains. Resume this task after the host perf
permission changes.
