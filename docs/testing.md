# Testing

`cargo test --workspace --all-features` tests reducers, bounded retention, protocol validation, ANSI classification, input mapping, and structural Ratatui rendering. Bridge framing can be exercised with `python3 bridge/ratabake_bridge.py`; it requires no Yocto checkout. Real Yocto validation is deliberately optional and should run `ratabake doctor`, bridge negotiation, `bitbake-layers show-layers`, and a parse-only command in a prepared environment.
