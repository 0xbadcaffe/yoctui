# Current Task

## Task

**ID:** RAW-SECURITY-001
**Title:** Verify Raw Mode has no shell-evaluation escape path
**Status:** IN_PROGRESS

## Objective

Reject shell operators and control corruption and prove ordinary execution
reaches only exact native argv or the separately typed PTY path.

## Dependencies

- `RAW-FAVORITE-UI-001` — DONE
- `RAW-SEARCH-001` — DONE
- `RAW-RESPONSIVE-001` — DONE
- `RAW-ARG-001` — DONE
- `RAW-JOB-001` — DONE
- `RAW-PTY-001` — DONE

## Relevant files

- `crates/yoctui-model/src/raw_mode.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/architecture.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Shell operators, substitutions, control bytes, and malformed arguments are
  rejected before process creation.
- Native argv and typed PTY execution remain separate and exact.
- Security tests cover hostile values and zero-spawn denial paths.

## Verification

```bash
cargo test --workspace --all-features raw_security
./scripts/verify-raw-security.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
