# Current Task

## Task

**ID:** CRATESIO-PACKAGE-001
**Title:** Prepare the crates.io package graph and release metadata
**Status:** IN_PROGRESS

## Objective

Prepare a bounded, accurately described crates.io release graph for `yoctui`
0.1.0 and its required internal crates. Keep test-only support crates private,
document registry installation, and verify package contents before any
irreversible upload.

## Dependencies

- `CRATESIO-BRIDGE-001` — DONE

## Relevant files

- `Cargo.toml`
- `crates/*/Cargo.toml`
- `README.md`
- `LICENSE`
- `scripts/verify-cratesio-package.sh`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Every public crate has accurate crates.io metadata and bounded contents.
- Internal path dependencies specify the exact 0.1 series needed for publish.
- Test-only and non-product support packages cannot be published accidentally.
- The README documents `cargo install yoctui --locked` as the primary install.
- Package archives and the binary help/version/embedded-bridge smoke pass.
- Baseline checks pass and documentation is committed.

## Verification

```bash
./scripts/verify-cratesio-package.sh
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
