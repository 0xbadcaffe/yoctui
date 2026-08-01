# Testing

`cargo test --workspace --all-features` tests reducers, bounded retention, protocol validation, ANSI classification, input mapping, and structural Ratatui rendering. `python3 -m pytest bridge/tests` covers bridge framing, mocked adapter shapes, event normalization, and deterministic live-harness preflight failures; those tests do not claim live compatibility.

Real Yocto validation is explicitly opt-in and runs through the production bridge:

```bash
YOCTUI_LIVE_BITBAKE=1 \
YOCTUI_LIVE_BUILD_DIR=/absolute/path/to/initialized/build \
./scripts/verify-live-bitbake.sh
```

The default safe normal operation first validates the selected recipe's
resolved provider/version, append count, task list, metadata sources, and
package outputs, then runs `base-files:do_listtasks`. The harness next starts
and immediately cancels `core-image-minimal`. Override these with
`YOCTUI_LIVE_TARGET`, `YOCTUI_LIVE_TASK`, and
`YOCTUI_LIVE_CANCEL_TARGET`. A bitbake-setup build may provide
`build/init-build-env`; otherwise source the environment first or set
`YOCTUI_OE_INIT_BUILD_ENV` to the checkout's `oe-init-build-env`.

`scripts/test-terminal.sh` starts Yoctui in a Linux pseudo-terminal, sends a quit key, and asserts that alternate-screen and cursor hide/show sequences are both emitted.

## Fuzzing

`fuzz/` contains cargo-fuzz targets for arbitrary protocol frames and bounded
log-retention operations. The checked-in corpus covers valid and malformed
JSON, an unsupported protocol version, an oversized frame, empty messages, and
retention pressure. Run the reproducible smoke budget with:

```bash
rustup toolchain install nightly
cargo install cargo-fuzz
./scripts/test-fuzz.sh
```

For a longer investigation, run either target directly and choose an explicit
time budget:

```bash
cargo +nightly fuzz run protocol_frames -- -max_total_time=3600 -max_len=4096
cargo +nightly fuzz run retained_logs -- -max_total_time=3600 -max_len=4096
```

Crashes are written below `artifacts/fuzz/` by the smoke script. A finite fuzz
run verifies the harness and its observed inputs; it is not an exhaustive
safety or compatibility claim.

## Stress and process trees

The deterministic stress gate drives 20,000 model log events through bounded
retention, frames and decodes 10,000 ordered protocol messages across
irregular chunks, and cancels a real Unix child process group containing a
TERM-resistant descendant. It checks retained counts/bytes/loss counters and
message order directly, then requires the exact descendant PID to disappear
within a bounded cancellation deadline.

```bash
./scripts/test-stress.sh
YOCTUI_STRESS_ITERATIONS=10 ./scripts/test-stress.sh
```

The default is three repetitions; values from 1 through 20 are accepted. The
process-tree case runs only on Unix, matching the runner's process-group
implementation.
# Completion gate

`./scripts/verify-completion.sh` is intentionally strict. It verifies the clean checkout, coverage thresholds, security checks, Python static checks, deterministic profiling workloads, and Flamegraph output. It exits with status 2 and names the missing prerequisite if a required completion tool has not been installed.
