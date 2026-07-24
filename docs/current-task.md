# Current task

## Active task

**ID:** CONFIG-001
**Title:** Configuration provenance workspace

## Objective

Complete the read-only-by-default configuration workspace with authoritative
values, provenance, scope, search/navigation actions, and safe editing.

## Required work

1. Inventory existing workspace variables, provenance chains, variable bridge
   queries, search, selection, source opening, BBMASK editing, and tests before
   adding behavior.
2. If the required Configuration scope is not atomic, split `CONFIG-001` into
   dependency-ordered child tasks and commit that governance change before
   implementation.
3. Show effective and unexpanded values where supported, global or
   recipe-specific scope, full provenance chains, overrides, and
   append/prepend/remove operations without deriving them from display text.
4. Add typed search, copy, defining-source navigation, and supported comparison
   workflows with explicit unavailable reasons.
5. Keep the workspace read-only by default. Any edit must use a dedicated
   preview-and-confirm dialog and refresh authoritative metadata afterward.
6. Preserve stable selection and safe responsive behavior across refresh,
   partial failure, empty data, and narrow terminals.
7. Add unit, bridge/fake-process, app, CLI, and TestBackend coverage appropriate
   to each atomic child task.

## Definition of done

- Every required Configuration field has a typed authoritative source or an
  explicit unavailable state.
- Search, copy, source opening, and supported comparison use typed actions.
- Editing is previewed, explicitly confirmed, and followed by refresh.
- Selection and focus remain stable across refresh and responsive modes.
- Live provenance is validated; mocked data alone is not completion evidence.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-ui config
cargo test -p yoctui-bitbake variable
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-001 — Complete Devtool lifecycle`
