# Current task

## Active task

**ID:** MAINT-001
**Title:** Advanced maintenance workflows

## Objective

Turn the broad Maintenance milestone into safe, typed, independently
verifiable workflows for sstate, PR/hash services, locked signatures, build
comparison, archives, and release engineering.

## Required work

1. Inspect the existing Maintenance model, UI, CLI routes, specifications, and
   tests before changing code.
2. Reconcile the current implementation with the Maintenance requirements in
   the UI and architecture contracts.
3. Because this parent is too large for one coherent commit, split it into
   atomic specification, model, adapter, rendering, CLI, and parent-gate tasks
   before implementation.
4. Set the highest-priority eligible child as current and continue
   immediately.

## Definition of done

- The Maintenance parent has explicit atomic children and dependency order.
- Each child has a concrete outcome and verification command.
- Specifications and human-readable status agree with the registry.
- The first eligible child becomes current.

## Verification

```bash
./scripts/verify-roadmap.sh
```

## Next task

Select the highest-priority eligible Maintenance child from
`docs/task-registry.toml`.
