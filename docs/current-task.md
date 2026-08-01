# Current Task

## Task

**ID:** MAINT-SERVICE-UI-001
**Title:** Add typed PR service export and import forms

## Objective

Implement model-owned, focus-trapped `e` PR export and `m` PR import entry
forms with exact initialized build/endpoint context and native helper side
effects, without executing a process in this task.

## Required work

1. Inspect existing `PrServiceRequest`, service diagnostics, capability state,
   generic confirmations, input mapping, and renderer before changing code.
2. Add one bounded typed form whose operation is fixed by its entry shortcut,
   whose build directory and endpoint are read-only authoritative metadata, and
   whose canonical `.conf`/`.inc` path is keyboard-editable and validated.
3. Map `e` and `m` only in Services. Keep export and import meaning distinct;
   show that both can stop a memory-resident server and invalidate cache, and
   that import changes PR data.
4. `Enter` on a valid form emits only a typed adapter-preview effect; `Esc`
   closes without side effects. Missing helper/build/endpoint keeps entry inert
   with the existing typed disabled reason.
5. Render the form, exact context, validation, and side-effect warning safely
   at all responsive boundaries, themes, and no-color mode.
6. Add reducer, app mapping, and TestBackend coverage for export, import,
   invalid extension/path, unavailable capability, and cancellation.

## Definition of done

- Services `e/m` forms are reachable only with exact capability/context.
- Form confirmation emits a typed preview request and never runs the helper.
- Focused and baseline verification pass.

## Verification

```bash
cargo test -p yoctui-model maintenance_service_workspace
cargo test -p yoctui-app maintenance_service_workspace
cargo test -p yoctui-ui maintenance_service_workspace
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Documentation updates

- Update `docs/ui-spec.md` in the same commit for exact form controls.
- Mark `MAINT-SERVICE-UI-001` `DONE` only after every command passes.
- Update `docs/implementation-status.md`.
- Replace this file with `MAINT-SERVICE-CLI-001`.

## Next task

`MAINT-SERVICE-CLI-001`
