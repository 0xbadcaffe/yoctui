# Current task

## Active task

**ID:** CONFIG-EDIT-WRITE-001
**Title:** Write and refresh previewed configuration edits

## Objective

Atomically apply the exact confirmed assignment to `conf/local.conf`, preserve
the selected detail on failure, and refresh its authoritative value on success.

## Required work

1. Inventory existing BBMASK writes, settings atomic persistence, variable
   detail loading, effect execution, error notifications, and CLI tests.
2. Accept only the exact typed request produced by the confirmation reducer;
   revalidate the allowlisted global identity, destination, value, and
   assignment at the filesystem boundary.
3. Replace an active assignment for the exact variable or append one when it
   is absent. Preserve unrelated content, comments, newline style, and file
   permissions.
4. Write through a same-directory temporary file and atomically rename it over
   `conf/local.conf`; clean up a failed temporary write without damaging the
   original.
5. On success, request the exact global `VariableIdentity` again and update
   the Inspector only from its authoritative backend response.
6. On write or refresh failure, preserve the prior selected detail and report
   an actionable notification.
7. Add reducer/app/CLI tests named `config_edit_write` covering replacement,
   append, validation, atomic failure, refresh success, and refresh failure.
8. Perform external validation without leaving the live Yocto configuration
   modified, and record the exact environment/result.
9. Update specifications and status documents where behavior changes.

## Definition of done

- Confirmed edits use a validated typed request and atomic local.conf update.
- Existing assignments are replaced without duplicating the active variable.
- Unrelated content and file permissions are preserved.
- Success refreshes the exact global detail; failures preserve prior state.
- Unit, integration, and external validation pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-model config_edit_write
cargo test -p yoctui-app config_edit_write
cargo test -p yoctui -- config_edit_write
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`DEVTOOL-JOBS-001 — Run Devtool operations as persistent jobs`
