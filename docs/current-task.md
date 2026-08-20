# Current Task

## Task

**ID:** INPUT-TEST-001
**Title:** Test every documented shortcut
**Status:** IN_PROGRESS

## Objective

Dispatch every documented next-generation shortcut through the real typed
input path and prove the authoritative keymap, contextual footer, and Help
catalog agree on its route and label.

## Dependencies

- `FOOTER-UI-001` — DONE
- `PALETTE-UI-001` — DONE

## Relevant files

- `crates/yoctui-e2e/`
- `crates/yoctui-app/`
- `crates/yoctui-model/`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Every authoritative shortcut is dispatched and produces its expected typed
  action or state transition.
- Function keys, global keys, prefix chords, contextual workspace keys, and
  modal routes are covered without generic shell fallbacks.
- Footer labels and Help documentation agree with the keymap catalog.
- Duplicate or shadowed documented bindings fail deterministically.

## Verification

```bash
cargo test -p yoctui-e2e next_generation_keymap
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
