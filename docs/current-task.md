# Current task

## Active task

**ID:** CONFIG-META-001
**Title:** Add authoritative typed variable detail

## Objective

Extend the protocol and backend boundary with authoritative selected-variable
detail, including BitBake provenance history and explicit unavailable fields.

## Required work

1. Inventory the current `GetVariable` command/event, backend `VariableValue`,
   bridge Tinfoil query, varhistory use, workspace summary maps, app
   normalization, and compatibility tests.
2. Define version-compatible typed variable detail carrying the variable name,
   optional recipe scope, effective value, unexpanded value, provenance
   operations/chain, override context, and field-level unavailability.
3. Obtain expanded and unexpanded values through supported datastore calls.
   Normalize `varhistory.variable()` entries into typed operations with file,
   line, operation, and value when BitBake supplies them; do not parse display
   strings in Rust.
4. Preserve backward compatibility for older bridge payloads and mocked server
   adapters. Missing fields remain `None` or empty according to the typed
   contract.
5. Normalize protocol detail through `yoctui-bitbake` into pure model state;
   stale recipe/name responses must not overwrite a different selection.
6. Add protocol round-trip, adapter, reducer, bridge fake-Tinfoil, malformed
   payload, and relevant failure tests named `config_metadata`.
7. Validate the detail query against the available live BitBake/Poky workspace
   and record the exact version, variable, scope, and returned fields. Do not
   claim unsupported fields from mocked tests.

## Definition of done

- Variable detail crosses protocol, backend, app, and reducer boundaries as
  typed data.
- Effective/unexpanded values, scope, provenance operations, and overrides are
  authoritative or explicitly unavailable.
- Old payloads and unsupported BitBake fields degrade safely.
- Failure and stale-response behavior is covered.
- Live Tinfoil provenance evidence is recorded.
- Task-specific and baseline verification pass.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-protocol config_metadata
cargo test -p yoctui-bitbake config_metadata
cargo test -p yoctui-model config_metadata
python3 -m pytest bridge/tests -k config_metadata
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

`CONFIG-UI-001 — Complete searchable Configuration Inspector`
