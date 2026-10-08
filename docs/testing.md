# Testing

Use ordinary checks, then the relevant boundary checks. Fixtures do not certify
live Yocto/boot/device/debugging. See [Development](development.md) for setup.

## Ordinary verification

```bash
cargo fmt --all --check
cargo test --locked --workspace --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
python3 -m unittest scripts/test_version_bump.py
python3 scripts/check-version-bump.py
python3 scripts/check-library-layout.py
```

Containers need --init/a subreaper for lifecycle tests. Private XDG roots isolate
state; only owned processes stop. CI pins Rust 1.97.0, Python 3.12, Ruff 0.15.22,
mypy 2.3.0, pytest 9.1.1/pytest-cov 7.1.0; bridge coverage >=75%. Static tooling
checks the full ordered-fragment runtime projection without importing BitBake.

## Terminal and UI checks

| Boundary | Commands |
| --- | --- |
| PTY/input/flow | `./scripts/test-terminal.sh`, `./scripts/test-tui-pty.sh`, `./scripts/test-tui-keymap.sh`, `./scripts/test-tui-flow.sh` |
| Snapshots/setup | scripts/test-tui-snapshots.sh; python3 scripts/test-environment-setup.py |
| PTY harness | python3 -m unittest scripts/test_pty_acceptance.py |
| Exact cells | ./scripts/verify-m21-concept-screens.sh |
| Rasters | ./scripts/render-m22-concept-screenshots.sh --check; python3 scripts/test-m22-concept-raster.py |
| README images | python3 scripts/render-readme-screenshots.py --check |

Build first; PTY harness honors CARGO_TARGET_DIR. Cover 80x24/100x30/160x48,
below-minimum and wide→narrow→small→wide resize, inspector off/on, focus traps,
inert disabled actions, prefix/writer ownership, detach/reconnect, and quit/restoration.
Failures retain bounded captures. [Design fixtures](design/README.md) explain goldens;
exact rasters use pinned Cairo 1.18.4/PyCairo 1.27.0/DejaVu on Ubuntu 26.04.

## Documentation checks

```bash
./scripts/check-docs.sh
./scripts/test-readme-quickstart.sh
./scripts/verify-design-contracts.sh
./scripts/verify-workbench-design.sh
./scripts/verify-utility-coverage.sh --catalog-only
```

Checks validate links/anchors/sections, shell syntax, CLI/headless/Doctor, and rasters;
they do not execute installation examples or initialize a live build.

## Live Yocto validation

```bash
YOCTUI_LIVE_BITBAKE=1 YOCTUI_LIVE_BUILD_DIR=/absolute/build ./scripts/verify-live-bitbake.sh
```

Default smoke queries metadata, completes base-files:do_listtasks, and starts/cancels
core-image-minimal—not a completed image. Override YOCTUI_LIVE_TARGET,
YOCTUI_LIVE_TASK, YOCTUI_LIVE_CANCEL_TARGET intentionally. Vendor layouts may require
YOCTUI_OE_INIT_BUILD_ENV/their setup wrapper.

For an already-built image, set YOCTUI_LIVE_BUILD_DIR, YOCTUI_LIVE_IMAGE_MANIFEST,
YOCTUI_LIVE_PKGDATA_DIR, YOCTUI_LIVE_IMAGE_ARTIFACT, YOCTUI_LIVE_MACHINE,
YOCTUI_LIVE_IMAGE, and optional retained YOCTUI_LIVE_IMAGE_ROOTFS, then run:

```bash
cargo test -p yoctui-bitbake --test live_rootfs -- --ignored
./scripts/test-compatibility-matrix.sh --evidence-only
```

Fresh workbench inputs use test-live-workbench-ux.sh/verify-live-workbench-ux-evidence.sh;
optional six-scene captures use test-live-m22-concept-parity.sh. Record exact identities,
commands/outcomes/limits/hashes/freshness. Missing prerequisites block the claim.

## Fuzz, stress, and sanitizers

| Gate | Requirements / command |
| --- | --- |
| Finite fuzz smoke | Nightly + cargo-fuzz: ./scripts/test-fuzz.sh |
| Stress | ./scripts/test-stress.sh; YOCTUI_STRESS_ITERATIONS=1–20 (default 3). |
| ASan/LSan | Linux x86_64, nightly + rust-src: ./scripts/test-sanitizers.sh |

Longer fuzz: `cargo +nightly fuzz run protocol_frames -- -max_total_time=3600 -max_len=4096`
(or retained_logs). Crashes go to artifacts/fuzz/.
Stress covers retention/chunks/TERM-resistant descendants; owned PIDs must disappear,
including zombies. Sanitizer diagnostics/nonzero workloads fail; LSan is separate.

## Completion gate

./scripts/verify-completion.sh combines ordinary/PTY/fuzz/stress/sanitizer/static/
security/coverage/Valgrind/profiling and opt-in live checks. It needs a clean checkout
and host prerequisites; status 2 identifies missing inputs. Separate board/boot
acceptance still applies. See [Profiling](profiling.md) and [Performance](performance.md).
