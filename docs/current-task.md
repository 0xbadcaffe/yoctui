# Current Task

## Task

**ID:** UI-STARTUP-DIAG-001
**Title:** Complete clean installed startup diagnostics
**Status:** IN_PROGRESS

## Objective

Run parent acceptance for bounded non-obscuring bridge diagnostics, preserved
global/workspace key routing, and the exact installed release's live Poky theme
workflow.

## Dependencies

- `UI-STARTUP-STDERR-001` — DONE
- `UI-FOCUS-ROUTING-001` — DONE
- `UI-STARTUP-LIVE-001` — DONE

## Relevant files

- `crates/yoctui-bitbake/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `scripts/test-live-workbench.sh`
- `README.md`

## Definition of done

- The full workspace and all-feature tests pass.
- Workspace Clippy is warning-free.
- Python bridge tests pass.
- Documentation and roadmap checks pass.
- The coherent live acceptance change is committed.

## Verification

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
