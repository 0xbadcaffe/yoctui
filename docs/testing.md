# Testing

`cargo test --workspace --all-features` tests reducers, bounded retention, protocol validation, ANSI classification, input mapping, and structural Ratatui rendering. `python3 -m pytest bridge/tests` covers bridge framing, mocked adapter shapes, event normalization, and deterministic live-harness preflight failures; those tests do not claim live compatibility.

Real Yocto validation is explicitly opt-in and runs through the production bridge:

```bash
YOCTUI_LIVE_BITBAKE=1 \
YOCTUI_LIVE_BUILD_DIR=/absolute/path/to/initialized/build \
./scripts/verify-live-bitbake.sh
```

The default safe normal operation is `base-files:do_listtasks`. The harness then starts and immediately cancels `core-image-minimal`. Override these with `YOCTUI_LIVE_TARGET`, `YOCTUI_LIVE_TASK`, and `YOCTUI_LIVE_CANCEL_TARGET`. A bitbake-setup build may provide `build/init-build-env`; otherwise source the environment first or set `YOCTUI_OE_INIT_BUILD_ENV` to the checkout's `oe-init-build-env`.

`scripts/test-terminal.sh` starts Yoctui in a Linux pseudo-terminal, sends a quit key, and asserts that alternate-screen and cursor hide/show sequences are both emitted.
# Completion gate

`./scripts/verify-completion.sh` is intentionally strict. It verifies the clean checkout, coverage thresholds, security checks, Python static checks, deterministic profiling workloads, and Flamegraph output. It exits with status 2 and names the missing prerequisite if a required completion tool has not been installed.
