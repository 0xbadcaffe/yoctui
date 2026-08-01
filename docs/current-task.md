# Current Task

## Task

**ID:** MAINT-SSTATE-CLI-001
**Title:** Route sstate readiness and cleanup forms

## Objective

Connect the typed Sstate forms to exact adapter previews while keeping candidate
discovery non-blocking and preserving the existing phrase, destructive
confirmation, fresh rediscovery, runner, and cancellation boundaries.

## Required work

1. Inspect the new preview effects and existing adapter/coordinator paths before
   changing code; do not duplicate command construction.
2. Revalidate the exact correlated capability before accepting either request.
3. Reconstruct readiness through `MaintenanceSstateCommandSpec::readiness` and
   dispatch the adapter-produced preview into the existing confirmation flow.
4. Run cleanup preview discovery through the exact adapter command in an
   independent non-blocking stage, retain bounded stdout/stderr, parse only the
   adapter result, and dispatch the exact candidate preview into the existing
   phrase flow. Preview discovery must never delete files.
5. Preserve navigation, replacement, timeout, nonzero, cancellation, loss,
   output bounds, and stale-correlation behavior. Execution must still rerun
   discovery and compare the candidate set immediately before deletion.
6. Add fake filesystem/process CLI tests for readiness preview, cleanup
   candidates, empty candidates, nonzero, timeout/loss, stale capability, and
   confirmation routing. Do not claim live cache safety.

## Definition of done

- `c` reaches an exact normal confirmation through the adapter.
- `d` reaches the exact phrase dialog only after successful typed discovery.
- Discovery failures are visible and cannot create a destructive preview.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui -- maintenance_sstate_workspace
cargo test -p yoctui-bitbake maintenance_sstate
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/architecture.md` only if coordinator ownership changes.
- Mark `MAINT-SSTATE-CLI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-SERVICE-UI-001`.

## Next task

`MAINT-SERVICE-UI-001`
