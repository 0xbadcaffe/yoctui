# Current Task

## Task

**ID:** TELEMETRY-COCKPIT-001
**Title:** Build a geeky live telemetry and progress cockpit
**Status:** IN_PROGRESS

## Objective

Turn live build monitoring into a dense, terminal-native cockpit while keeping
every number authoritative and every layout safe. Add CPU, memory, disk, load,
history, task-velocity, ETA, and higher-resolution task progress visuals.

## Relevant files

- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/product-roadmap.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Host telemetry carries optional total/available memory, filesystem capacity,
  logical CPU count, and fixed-point 1/5/15-minute load averages.
- CLI parsers reject malformed and inconsistent procfs values without panics or
  fabricated zeroes and have normal plus failure-path tests.
- Reducer-owned CPU and memory histories retain at most 60 valid samples.
- Dashboard renders responsive semantic CPU, RAM, and disk gauges plus bounded
  history sparklines and load labels when vertical space permits.
- Dashboard and Tasks expose honest average completed-task velocity and ETA only
  when elapsed time and authoritative totals support them.
- Determinate task bars use bounded fractional-cell rendering; unknown progress
  remains visibly unknown and animated according to motion settings.
- Focused tests, baseline checks, documentation, and roadmap checks pass.
- The implementation and final task-state updates are committed coherently.

## Verification

```bash
cargo test -p yoctui-model host_telemetry
cargo test -p yoctui telemetry
cargo test -p yoctui-ui telemetry
cargo test -p yoctui-ui task_progress
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
