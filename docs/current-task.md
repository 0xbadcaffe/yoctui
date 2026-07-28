# Current task

## Active task

**ID:** WIC-ADAPTER-RUNNER-001
**Title:** Run Wic creation and scan exact outputs

## Objective

Own one cancellable Wic creation process with bounded typed output and return
only exact new generated files beneath the requested output directory.

## Required work

1. Inspect the QEMU runner and image scanner before writing code; reuse bounded
   line, process-group cancellation, and canonical path invariants.
2. Snapshot the exact canonical output directory before spawn without following
   symlinks and with deterministic entry/time bounds.
3. Start one `WicCreateCommandSpec` in the active build directory with an
   independent child process group, bounded stdout/stderr channel and lines,
   duplicate rejection, and explicit starting/started events.
4. On successful exit, rescan the same directory and return only new or changed
   canonical regular non-symlink files with typed kind, size, and modification
   time. Preserve empty and partial scan results honestly.
5. Emit distinct nonzero failure, graceful/forced cancellation, cancellation
   rejection, stream/process loss, timeout, and output-scan failure events.
6. Add fake-process/filesystem tests named `wic_adapter_runner` for exact
   working directory/arguments, success/empty/partial output, nonzero exit,
   duplicate start, stream bounds, cancellation, and loss.
7. Add mechanical app normalization, run focused and baseline checks, then mark
   the child done and hand off to the `WIC-ADAPTER-001` parent gate.

## Definition of done

- One process is owned and polled without blocking the UI.
- Stream and output inventories remain bounded and typed.
- Only exact new canonical files under the requested root are returned.
- Success, failure, cancellation, rejection, and loss remain distinct.
- Fake coverage is not presented as live compatibility.
- Focused and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-bitbake wic_adapter_runner
cargo test -p yoctui-app wic_adapter_runner
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
