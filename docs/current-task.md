# Current Task

## Task

**ID:** HARDEN-001
**Title:** Complete test and analysis matrix

## Objective

Close the required hardening milestone with verified property, fuzz, stress,
terminal, sanitizer, Valgrind, profiling, and flamegraph coverage appropriate
to the repository.

## Required work

1. Inspect the existing hardening implementation, tests, scripts, CI, and
   documentation before adding anything.
2. Reconcile the task against `docs/product-roadmap.md`, `docs/ui-spec.md`, and
   `docs/architecture.md` and identify the exact remaining gaps.
3. Because this parent outcome spans unrelated verification techniques, split
   each missing gap into one concrete atomic child task with explicit files,
   dependencies, definition of done, and verification commands before
   implementation.
4. Execute every child in dependency order, retaining platform/tool
   limitations explicitly rather than weakening verification.
5. Run the complete workspace test and clippy gate after all children pass.

## Definition of done

- Every required hardening technique has verified implementation or an exact
  documented external blocker with a follow-up validation command.
- All atomic hardening child tasks are `DONE` or correctly `BLOCKED`.
- Workspace tests and warning-free clippy pass.
- `HARDEN-001` is marked `DONE` only when its required completion gate passes.

## Verification

```bash
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Split missing independent outcomes in `docs/task-registry.toml` before code.
- Keep `docs/implementation-status.md` synchronized with each child.
- Replace this file with the next eligible atomic child after the split.

## Next task

To be selected by the hardening gap audit.
