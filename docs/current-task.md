# Current Task

## Task

**ID:** RAW-PARAM-001
**Title:** Implement typed Raw parameter validation
**Status:** IN_PROGRESS

## Objective

Define bounded user values for every Raw parameter kind, validate required and
optional fields without shell interpretation, and retain typed values for
later exact argv construction.

## Dependencies

- `RAW-CATALOG-MODEL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Recipe, image, target, task, UI, file, integer, text, and multiconfig values
  have closed typed validation rules and explicit byte bounds.
- Required fields reject empty input; optional fields normalize empty input to
  absence without inventing a value.
- Control characters, shell metacharacters, traversal, invalid numeric ranges,
  and kind/definition disagreements fail closed.
- Tests cover every kind, Unicode/length boundaries, optional values, and
  representative invalid input.

## Verification

```bash
cargo test -p yoctui-model raw_parameter
cargo clippy -p yoctui-model --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
