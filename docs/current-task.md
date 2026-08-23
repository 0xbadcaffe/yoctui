# Current Task

## Task

**ID:** RAW-ARG-001
**Title:** Implement bounded expert argv editor
**Status:** IN_PROGRESS

## Objective

Implement the Raw form's bounded `Additional arguments` editor and tokenizer
so quoted user intent becomes native argv elements without invoking or
emulating a shell.

## Dependencies

- `RAW-PARAM-001` — DONE
- `UX-POPUP-EDITOR-005` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Unquoted, single-quoted, and double-quoted input produces deterministic
  native argv elements without retaining grouping quotes.
- Backslash escapes only the next ordinary character under the documented
  grammar; it performs no expansion or substitution.
- Unterminated quotes/escapes, controls, empty option names, excess argument
  count/element/aggregate bytes, and documented shell operators are rejected
  with typed validation errors.
- Empty input is a valid empty argv suffix while quoted empty arguments remain
  explicit native empty elements.
- App mapping uses the typed model editor/tokenizer and never constructs a
  shell command string.
- Model and app tests cover normal quoting/escaping, exact boundaries, empty
  arguments, Unicode, every rejected operator, and failure/replacement paths.

## Verification

```bash
cargo test -p yoctui-model raw_argv
cargo test -p yoctui-app raw_argv
cargo clippy -p yoctui-model -p yoctui-app --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
