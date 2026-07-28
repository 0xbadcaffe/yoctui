# Current task

## Active task

**ID:** SDK-001
**Title:** SDK build and artifact workflow

## Objective

Provide typed standard-SDK and extensible-SDK build, artifact inspection,
publication, and extracted-SDK workflows without turning the shell escape hatch
into the primary UX.

## Required work

1. Inspect existing build jobs, Images artifacts, background-job lifecycle,
   command execution adapters, Navigator/workspace composition, UI
   specification, and roadmap dependencies before editing.
2. Reconcile the broad parent task with the repository's one-coherent-commit
   task discipline. Split it into independently verifiable model/adapter,
   rendering, CLI integration, and parent-gate children if those outcomes do
   not already exist.
3. Set the first eligible child task current and commit the governance split
   before implementation.
4. Preserve the architecture dependency direction and use typed identities,
   previews, effects, lifecycle events, and bounded output throughout.

## Definition of done

- The SDK parent is decomposed into coherent dependency-ordered tasks.
- Existing behavior is identified and reused.
- The first implementation child has explicit files, behavior, tests,
  documentation updates, and verification commands.

## Verification

```bash
./scripts/verify-roadmap.sh
```

## Next task

Select the first eligible SDK child from `docs/task-registry.toml`.
