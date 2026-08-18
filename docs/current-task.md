# Current Task

## Task

**ID:** UI-LITERAL-001
**Title:** Complete literal reference workbench
**Status:** IN_PROGRESS

## Objective

Close the literal reference-workbench milestone after every atomic visual,
interaction, and live-Poky acceptance task has passed.

## Dependencies

- `UI-LITERAL-HARNESS-001` — DONE
- `UI-LITERAL-SHELL-001` — DONE
- `UI-LITERAL-NAV-001` — DONE
- `UI-LITERAL-COCKPIT-001` — DONE
- `UI-LITERAL-UX-001` — DONE
- `UI-LITERAL-LIVE-001` — DONE

## Relevant files

- `docs/current-task.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Every M16 child task is `DONE` with its required evidence.
- The strict terminal-cell golden and live Poky gate remain passing.
- Workspace tests, Clippy, Python bridge tests, docs, and roadmap checks pass.
- The milestone state and human-readable handoff are current.

## Verification

```bash
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
