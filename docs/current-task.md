# Current Task

## Task

**ID:** RAW-COMPAT-001
**Title:** Verify dynamic Raw availability across BitBake fixtures
**Status:** IN_PROGRESS

## Objective

Exercise older, Wrynose 2.18, unknown-future, unavailable, limited,
replacement, stale, and denial-with-zero-spawn behavior.

## Dependencies

- `RAW-FAVORITE-UI-001` — DONE
- `RAW-SEARCH-001` — DONE
- `RAW-RESPONSIVE-001` — DONE
- `RAW-CAP-PROBE-001` — DONE
- `RAW-FORM-UI-001` — DONE
- `RAW-SECURITY-001` — DONE

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

- Fixture authority snapshots classify all five availability states exactly.
- Replacement and stale projections close unsafe forms without spawn effects.
- Compatibility tests cover old, current, and unknown-future evidence.

## Verification

```bash
cargo test --workspace --all-features raw_compatibility
./scripts/verify-compatibility.sh
cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
