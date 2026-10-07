# Testing

## Hosted CI environment

Release jobs pin Rust1.97.0, matching the compiler used to validate/package
0.1.315. Strict all-target/all-feature Clippy retains `-D warnings`.
Floating stable1.99 hit the upstream
[async_trait macro false-positive](https://github.com/rust-lang/rust-clippy/issues/17529);
no blanket warning suppression or product-source workaround is introduced.
Compiler upgrades require a separate validated CI update.

The production bridge loads eleven ordered fragments into shared globals.
Ruff checks their complete ordered source projection, the real loader and every
test module; mypy additionally represents the exact runtime-bound functions as
class methods, retaining their signatures/bodies and existing narrow native-bb
import directives. Neither projection runs the bridge or omits a fragment.
Mutation tests reject omitted fragments, unsupported bindings and undefined
names. The formatter still checks the actual files, and pytest retains75%
subprocess coverage. Python formatting may retain a release version only when
its runtime AST is identical; function/literal changes still require a bump.

Evidence validation jobs fetch full Git history so retained source commits can
be checked for ancestry. The documentation/test job uses Ubuntu 26.04 with
Cairo 1.18.4, PyCairo 1.27.0 and the manifest-pinned DejaVu fonts; both screenshot
galleries must still reproduce byte-for-byte. Bridge lint and coverage target
the bundled production bridge plus its tests.

The container explicitly installs rustfmt and Clippy and trusts only its exact
mounted checkout in system Git configuration. Docker's `--init` reaps orphaned
fixture descendants so the process-cancellation tests retain their normal-host
lifecycle requirements and unchanged deadlines.

The Python quality tools are pinned to the release-validated versions:
Ruff 0.15.22, mypy 2.3.0, pytest 9.1.1 and pytest-cov 7.1.0. CI previously
installed floating versions, so Ruff 0.16.6 applied different rules from local
verification. Keep tool upgrades explicit and validate their policy/formatting
changes together; the production-source coverage requirement stays at 75%.

The real PTY startup/keymap gates wait for the rendered onboarding footer,
dismiss it, then observe the workbench. They keep draining output during exit
and answer the displayed quit confirmation before requiring successful exit
and terminal restoration. The initial window-title escape sequence is not
treated as a rendered frame.

All five release PTY probes now share private XDG/Yocto environment setup and
explicit `YOCTUI_TERMINAL_GRAPHICS=none`, so a graphical desktop cannot move
the client into an unobserved XTerm. They resolve the actual `CARGO_TARGET_DIR`
binary (including relative targets); a missing configured binary fails instead
of falling back to an old repository binary. The documentation headless probe
uses the same binary resolver. This changes test setup, not normal graphics
handoff behavior.

The terminal compositor answers cursor-position requests from its observed
cursor, including fragmented requests, and drains the completed frame before
sending the next key. This prevents a partial footer from coalescing Escape
with a cursor report. Startup/quit bounds remain eight/three seconds; snapshot
onboarding dismissal and task routing retain their 0.5/two-second bounds.
Resize coverage requires the actual below-80x24 warning and wide recovery;
forced termination or exit status one is no longer an accepted successful run.
All probes require alternate-screen entry and restoration.
The first-frame metric includes the bounded 50 ms quiet-output synchronization
used to finish the observed frame and answer its queries. Earlier tracked samples
that measured only terminal setup escapes and exited with status one are not
valid startup-performance baselines; do not compare them as a speed regression.

Snapshot captures use a real isolated Yoctui daemon with a temporary executable
initializer, configuration files and the existing deterministic bridge fixture.
The fixture BitBake command accepts only `--version`, never an image build.
The wide snapshot must show the current unset-target Tasks title, footer routes
and actual connected daemon. Capture occurs before the quit overlay; version
identity is retained. These tests are offline integration evidence, not live
Yocto/OpenBMC, optimized-installation or post-reboot certification. Only their
owned fixture daemon/client is stopped. Thirteen positive/negative helper tests
run at the release-quality boundary:

```bash
python3 -m unittest scripts/test_pty_acceptance.py
./scripts/verify-release-quality.sh
```

## Environment path setup regression

`cargo test --workspace environment_setup` covers the typed draft, Unicode and
path bounds, stale replies, directory errors, hidden folders, symlinks, split
source layout, key routing and narrow-terminal selection. After
`cargo build --locked -p yoctui`, run
`python3 scripts/test-environment-setup.py`. The real 120x32 PTY test uses private
XDG roots and a fake source tree: manual bracketed paste, Source/Build browsing,
child/parent navigation and saving must work without a daemon, source-script
execution or build initialization. It is startup/UI evidence, not live BitBake
compatibility evidence. The completion script invokes this test explicitly.

## Overview visualization regressions

The focused suite covers all eight responsive Insights empty/data states,
CycloneDX and legacy manifest parsing, rootfs pie/table breakpoints, and
recipe-root reachability for process-backend task graphs:

```bash
cargo test -p yoctui-model overview
cargo test -p yoctui-bitbake security_report_parses_cyclonedx_and_legacy_image_manifest
cargo test -p yoctui-bitbake dependency_graph_dot_parser
cargo test -p yoctui-ui overview_insights
cargo test -p yoctui-ui ux_rootfs_packages_pair
cargo test -p yoctui-ui ux_dependency_graph
```

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

An already-built image can exercise the production Rootfs adapter directly.
All variables are required except `YOCTUI_LIVE_IMAGE_ROOTFS`, which is optional
when work cleanup removed the logical root:

```bash
YOCTUI_LIVE_BUILD_DIR=/absolute/build \
YOCTUI_LIVE_IMAGE_MANIFEST=/absolute/image.manifest \
YOCTUI_LIVE_PKGDATA_DIR=/absolute/build/tmp/pkgdata/machine \
YOCTUI_LIVE_IMAGE_ARTIFACT=/absolute/image.ext4.zst \
YOCTUI_LIVE_MACHINE=qemux86-64 \
YOCTUI_LIVE_IMAGE=core-image-kernel-dev \
cargo test -p yoctui-bitbake --test live_rootfs -- --ignored
```

The ignored test is never treated as fixture evidence: it requires exact live
paths and asserts that generated manifest/pkgdata becomes package composition
without a screen-level failure.

`scripts/test-terminal.sh` starts Yoctui in a Linux pseudo-terminal, sends a quit key, and asserts that alternate-screen and cursor hide/show sequences are both emitted.

## Live workbench evidence

Fresh live workbench evidence can be captured and verified with:

```bash
./scripts/test-live-workbench-ux.sh
./scripts/verify-live-workbench-ux-evidence.sh
./scripts/verify-compatibility.sh
```

The live commands require a complete newly supplied workspace/capture bundle;
the old tracked bundle was removed. They retain capability, source identity,
scenario, checksum and freshness checks. Documentation validation separately
reproduces the three README SVGs from the retained source captures.

Deterministic screenshot checks validate current production cells and rasters:

```bash
./scripts/render-m22-concept-screenshots.sh --check
python3 scripts/test-m22-concept-raster.py
python3 scripts/verify-m21-concept-screens.py --fixtures-only
python3 scripts/test-m21-concept-screen-verifier.py
python3 scripts/render-next-generation-ui-screenshots.py --check
```

The fixture checks validate dimensions, symbols, styles, semantic anchors,
hashes and raster-source identity. The SVG check reproduces the three README
live illustrations from their retained source text and manifest. The retired
six-scene live gallery is outside documentation validation. None of these
checks establishes fresh live Yocto acceptance.

Fresh live bundles can still be captured and checked with
`./scripts/test-live-m22-concept-parity.sh` and its explicit live-evidence
verifier. They require new complete inputs; missing captures must fail.

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

## Yocto host Python troubleshooting

BitBake hosttools must resolve a working Python interpreter in their restricted
PATH. A pyenv shim pointing back to the same hosttools link can recurse rather
than execute Python. Check interpreter and hosttools resolution in the actual
initialized build environment. Preserve explicit virtualenv/pyenv choices and
avoid changing distribution-owned interpreters or another user's configuration.
See [pyenv issue 2696](https://github.com/pyenv/pyenv/issues/2696) and the
[upstream shim-alias fix](https://github.com/pyenv/pyenv/pull/3375).
