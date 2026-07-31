# Current task

## Task

**ID:** MAINT-MODEL-001
**Title:** Model typed Maintenance state and operations

## Objective

Add pure, bounded, exact-identity model state and reducer behavior for the
four-view Maintenance workspace without host, filesystem, process, or raw-text
access.

## Required work

1. Add a `maintenance` model module owning:
   - fixed Sstate, Services, Release, and Integrations views
   - capability and configured-metadata snapshots
   - canonical typed input/evidence identities
   - sstate readiness and cleanup drafts/previews
   - PR/hash diagnostics and PR import/export requests
   - locked-cache, build-comparison, and Git-archive requests
   - optional integration detection
   - one stable bounded operation session and exact terminal outcomes
2. Add deterministic validation for required fields, cleanup phrase and exact
   candidate identity, destructive/network confirmation, stale correlations,
   cancellation, timeout, failure, loss, and replaceable evidence.
3. Integrate first-class `Screen::Maintenance`, state ownership, navigation,
   actions, effects, and backend normalization in `yoctui-model` and
   `yoctui-app`; remove the Maintenance Navigator alias to BBMASK.
4. Preserve existing Signatures, Security, QA, and recipe patch-review
   ownership and expose only typed navigation actions to them.
5. Add focused unit and reducer tests for normal and relevant failure paths.
6. Keep all collections and output bounded and deterministic.

## Definition of done

- Maintenance is first-class typed state with no host/process access.
- Every specified operation can be represented without free-form shell text.
- Validation and reducer tests cover exact identities, previews, confirmations,
  lifecycle, stale results, cancellation, every terminal outcome, evidence
  replacement, selection, and cross-workspace routes.
- Existing workspace behavior and baseline verification remain green.

## Verification

```bash
cargo test -p yoctui-model maintenance_workflow
cargo test -p yoctui-app maintenance_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/ui-spec.md` with any intentional behavior change.
- Update `docs/architecture.md` with any changed component boundary.
- Mark `MAINT-MODEL-001` `DONE` only after verification passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-SSTATE-ADAPTER-001`.

## Next task

`MAINT-SSTATE-ADAPTER-001`
