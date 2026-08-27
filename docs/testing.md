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

## Live workbench evidence

The checked-in M21 evidence is verified without rebuilding Yocto by:

```bash
./scripts/test-live-workbench-ux.sh
./scripts/verify-live-workbench-ux-evidence.sh
./scripts/verify-compatibility.sh
```

The first command drives the release binary through real PTYs against the
recorded live workspace and checks semantic menus, progress, completion,
failure, manifest/pkgdata/rootfs, context-terminal, and reconnect screens. The
second validates scenario coverage, hashes, freshness, source ancestry, package
evidence, and the current latest/older release anchors. `check-docs.sh` also
re-renders the three live SVGs from their semantic captures and fails when a
checked-in visual is stale.

Regeneration is opt-in and requires a disposable supported Yocto host. The live
harness accepts an exact already-built release binary so an older supported
container does not need the development Rust toolchain:

```bash
YOCTUI_LIVE_COMPLETE=1 \
YOCTUI_LIVE_SOURCE=/absolute/path/to/poky \
YOCTUI_LIVE_PREBUILT_BINARY=/absolute/path/to/release/yoctui \
./scripts/test-live-next-generation-ui.sh
python3 scripts/render-next-generation-ui-screenshots.py
```

Record the host distribution/libc, source revision, binary hash, machine,
target, and every terminal outcome. The current run used Ubuntu 24.04.4 with
glibc 2.39 because the development host is outside the tested Yocto host
matrix. A failed host build is never converted into passing evidence.

## Documentation validation

`./scripts/check-docs.sh` validates every tracked repository Markdown link and
used fragment locally without network access. It also requires the installation,
operator, compatibility, testing, profiling, architecture, protocol, and UI
documents and their critical workflow/troubleshooting sections. The gate checks
current CLI help, runs the no-Yocto headless workload and doctor with isolated
configuration, and runs `bash -n` over a sorted list of every tracked shell
script. A developer session can therefore neither inject a remembered build
target nor turn documentation validation into a BitBake build.

```bash
./scripts/check-docs.sh
```

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

## Sanitizers

The sanitizer gate currently supports Linux x86_64 and requires nightly Rust
plus its `rust-src` component. It rebuilds the standard library and selected
workspace crates in isolated `target/sanitizers/` directories, runs the model
and protocol stress cases under AddressSanitizer and LeakSanitizer, then runs
the production headless bridge lifecycle under AddressSanitizer.

```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
./scripts/test-sanitizers.sh
```

AddressSanitizer leak detection is disabled because leak checking is performed
separately by LeakSanitizer. The headless workload uses the native process
backend to cover CLI startup, backend selection, workspace inspection, and
shutdown; protocol framing is covered by the separately instrumented stress
test. Any sanitizer diagnostic or nonzero workload exit fails the gate.
# Completion gate

`./scripts/verify-completion.sh` is intentionally strict. It verifies the clean checkout, ordinary tests, pseudo-terminal lifecycle, finite fuzz smoke, repeated stress/process-tree behavior, ASan/LSan, coverage thresholds, security checks, Python static checks, Valgrind, deterministic profiling, Flamegraph output, and the opt-in live BitBake gate. It exits with status 2 and names a missing prerequisite or host permission; no hardening check is silently skipped.
