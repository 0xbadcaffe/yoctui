# Testing

`cargo test --workspace --all-features` tests reducers, bounded retention, protocol validation, ANSI classification, input mapping, and structural Ratatui rendering. Bridge framing can be exercised with `python3 bridge/yoctui_bridge.py`; it requires no Yocto checkout. Real Yocto validation is deliberately optional and should run `yoctui doctor`, bridge negotiation, `bitbake-layers show-layers`, and a parse-only command in a prepared environment.

`scripts/test-terminal.sh` starts Yoctui in a Linux pseudo-terminal, sends a quit key, and asserts that alternate-screen and cursor hide/show sequences are both emitted.
# Completion gate

`./scripts/verify-completion.sh` is intentionally strict. It verifies the clean checkout, coverage thresholds, security checks, Python static checks, deterministic profiling workloads, and Flamegraph output. It exits with status 2 and names the missing prerequisite if a required completion tool has not been installed.
