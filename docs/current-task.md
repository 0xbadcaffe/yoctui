# Current task

## Active task

**ID:** CONFIG-META-001
**Title:** Add authoritative typed variable detail

## Objective

Complete the required live Tinfoil validation for the already implemented
typed configuration-variable detail path.

## Required work

1. Reinspect the typed protocol, bridge, adapter, app, and model implementation
   without reimplementing behavior that already exists.
2. Run the focused configuration metadata tests.
3. Against the existing qemux86-64 Yocto build, use Tinfoil in
   configuration-only mode to query `MACHINE` and record:
   - expanded value
   - unexpanded value
   - normalized variable history operations
   - active `OVERRIDES`
   - BitBake version
4. Confirm the live values are representable by the existing typed payload and
   model identity without widget-side parsing.
5. Record the exact live evidence in the registry and implementation status.

## Definition of done

- Focused and baseline verification pass.
- Live Tinfoil returns expanded/unexpanded `MACHINE`, history, overrides, and
  version data from the existing build.
- The typed bridge path can represent the live result honestly.
- Registry/status documents are updated and the next eligible task is active.

## Verification

```bash
cargo test -p yoctui-protocol config_metadata
cargo test -p yoctui-bitbake config_metadata
cargo test -p yoctui-app config_metadata
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
