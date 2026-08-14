# Current Task

## Task

**ID:** BINARY-ONE-RUST-001
**Title:** Preserve one Rust-native Yoctui product
**Status:** IN_PROGRESS

## Objective

Keep daemon/client as one installed Rust Yoctui package or documented
same-workspace helpers; no Electron or browser runtime.

## Verification

```bash
cargo test -p yoctui binary_product
./scripts/check-docs.sh
```
