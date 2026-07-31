# Current task

## Active task

**ID:** MAINT-SPEC-001
**Title:** Specify typed Maintenance workflows

## Objective

Define the authoritative product and architecture contract for one safe,
responsive Maintenance destination covering sstate readiness and protected
cleanup, PR/hash service diagnostics, locked signatures, build comparison,
Git archives, and optional release integrations.

## Required work

1. Inspect current official tool interfaces and the initialized workspace
   metadata available to Yoctui.
2. Define exact capability, identity, selection, preview, confirmation,
   execution, evidence, cancellation, timeout, failure, and loss behavior for
   each required family.
3. Keep internal PR/hash/worker services observational; never launch them as a
   normal workflow.
4. Require destructive cache actions to show exact affected paths and a
   separate explicit confirmation.
5. Reuse completed Signatures, Security, QA, and patch-review workflows rather
   than duplicating them in Maintenance.
6. Define responsive layout, focus-trapped dialogs, complete full/compact
   footers, model/adapter/app/UI/CLI ownership, and live-validation limits.
7. Update `docs/ui-spec.md`, `docs/architecture.md`, the registry, and status
   together.

## Definition of done

- Maintenance behavior and safety are explicit enough to implement without
  inventing layouts, keys, task names, paths, or command vectors.
- Component ownership and fake-versus-live evidence boundaries are explicit.
- Roadmap validation passes.

## Verification

```bash
./scripts/verify-roadmap.sh
```

## Next task

`MAINT-MODEL-001`
