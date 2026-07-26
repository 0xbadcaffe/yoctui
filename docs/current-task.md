# Current task

## Active task

**ID:** DEVTOOL-JOB-SPEC-001
**Title:** Define typed Devtool operation commands

## Objective

Define validated typed Devtool operations and exact shell-free argument
construction shared by future background execution and workflow reducers.

## Required work

1. Inventory current Devtool request structs, reducer effects, direct CLI
   argument lists, metadata identities, and validation helpers.
2. Add a typed operation enum covering modify, update-recipe, finish,
   deploy-target, undeploy-target, and reset.
3. Carry each operation's required recipe, destination, or target as typed
   fields; reject empty/control-bearing recipe and target values, relative
   finish destinations, and irrelevant fields.
4. Keep process-independent validation in `yoctui-model`.
5. Translate each validated operation to an exact `devtool` argument vector
   in `yoctui-bitbake`; never construct a shell command string.
6. Preserve paths as `OsString`/`PathBuf` so non-UTF-8 destinations do not
   require lossy conversion.
7. Add model and adapter tests named `devtool_job_spec` for every operation
   and relevant invalid input.
8. Update architecture documentation for the typed command boundary.

## Definition of done

- Every supported lifecycle operation has one typed representation.
- Validation is deterministic and independent of process execution.
- Exact argument vectors are shell-free and preserve destination paths.
- Focused and baseline verification pass.
- Registry/status documents are updated and the runner child becomes active.

## Verification

```bash
cargo test -p yoctui-model devtool_job_spec
cargo test -p yoctui-bitbake devtool_job_spec
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-JOB-RUNNER-001 — Add cancellable Devtool process streaming`
