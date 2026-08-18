# Current Task

## Task

**ID:** UI-LITERAL-HARNESS-001
**Title:** Add strict cell and style visual acceptance
**Status:** IN_PROGRESS

## Objective

Make the approved `160x48` Tasks workbench deterministic and compare every
application-controlled terminal cell, including symbol, colors, and modifiers.

## Dependencies

- `UI-LITERAL-SPEC-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-ui/tests/golden/`
- `scripts/test-tui-snapshots.sh`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- A typed deterministic fixture renders the complete reference scene.
- Rendering accepts an injected clock for deterministic elapsed values.
- The canonical artifact records symbols, colors, and modifiers for all cells.
- A mismatch identifies the first changed coordinate and expected/actual cell.
- Golden updates are explicit and never automatic during normal verification.

## Verification

```bash
cargo test -p yoctui-ui literal_reference
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
