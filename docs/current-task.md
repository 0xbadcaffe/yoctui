# Current Task

## Task

**ID:** MAINT-SERVICE-CLI-001
**Title:** Route PR service export and import forms

## Objective

Connect typed PR export/import forms to exact adapter previews and the existing
fresh-inspecting independent Maintenance runner.

## Required work

1. Route `PreviewPrService` through a fresh correlated capability inspection.
2. Reconstruct only the exact typed export/import request with
   `pr_service_command`; reject changed helper, build, endpoint, file, or
   capability identity visibly.
3. Dispatch the adapter preview into the existing generic confirmation flow;
   do not spawn before confirmation.
4. Preserve import destructive styling, export destination replacement
   warning, exact indexed vector, output, evidence, navigation, cancellation,
   and every terminal outcome through the existing runner.
5. Add fake filesystem/process CLI tests for export and import previews,
   successful export evidence, nonzero failure, stale identity, and input
   rejection. Do not claim live PR database compatibility.

## Definition of done

- Both forms reach exact adapter-owned confirmations and confirmed execution.
- Export installs exact validated evidence only after success.
- Failure cannot erase prior evidence or mutate UI-owned state directly.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui -- maintenance_service_workspace
cargo test -p yoctui-bitbake maintenance_service
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/architecture.md` only if coordinator ownership changes.
- Mark `MAINT-SERVICE-CLI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-RELEASE-UI-001`.

## Next task

`MAINT-RELEASE-UI-001`
