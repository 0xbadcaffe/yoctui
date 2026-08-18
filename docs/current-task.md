# Current Task

## Task

**ID:** UI-STARTUP-STDERR-001
**Title:** Keep bridge diagnostics out of the alternate screen
**Status:** IN_PROGRESS

## Objective

Continuously drain bridge standard error into a bounded diagnostic tail so
BitBake startup notes, warnings, and shutdown traces cannot overwrite the TUI,
while preserving useful context for a genuine bridge startup failure.

## Dependencies

- `UI-STARTUP-DIAG-SPEC-001` — DONE

## Relevant files

- `crates/yoctui-bitbake/src/lib.rs`
- `docs/architecture.md`
- `docs/ui-spec.md`

## Definition of done

- Bridge stderr is piped and continuously drained under a fixed byte bound.
- Ordinary stderr output never reaches the inherited terminal.
- A failed bridge handshake includes the bounded diagnostic context.
- Focused tests cover normal stderr, failure diagnostics, and truncation.

## Verification

```bash
cargo test -p yoctui-bitbake bridge_stderr
cargo fmt --all --check
cargo clippy -p yoctui-bitbake --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
