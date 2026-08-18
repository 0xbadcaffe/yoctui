# Current Task

## Task

**ID:** COMPAT-SPEC-001
**Title:** Specify Yocto-feature-correlated functionality
**Status:** IN_PROGRESS

## Objective

Define the complete compatibility contract for dynamically enabling,
disabling, adapting, or replacing functionality according to authoritative
evidence from the connected Yocto/OpenEmbedded/BitBake environment.

## Dependencies

- `UI-WIDE-RAIL-001` — DONE

## Relevant files

- `docs/compatibility.md`
- `docs/compatibility-matrix.md`
- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Authoritative environment identity and explicit Unknown semantics are defined.
- Capability detection versus fallback inference and precedence are defined.
- Supported/minimum/latest/future/unsupported/degraded release policy is defined.
- Availability states, version-specific alternatives, and evidence rules are defined.
- UI, daemon, protocol, cache, and command authority boundaries are unambiguous.
- Documentation and registry verification pass.

## Verification

```bash
./scripts/check-docs.sh
./scripts/verify-compatibility.sh --structure-only
./scripts/verify-roadmap.sh
```
