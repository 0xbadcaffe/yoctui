# Current Task

## Task

**ID:** RAW-SPEC-001
**Title:** Specify Raw Mode UX execution safety favorites and capability contract
**Status:** IN_PROGRESS

## Objective

Define the authoritative UI and architecture contract for the Raw BitBake
Command Workbench before changing runtime behavior.

## Dependencies

- `RAW-REF-001` — DONE

## Relevant files

- `docs/ui-spec.md`
- `docs/architecture.md`
- `docs/product-roadmap.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

 - Raw Mode hierarchy, focus, search, form, preview, output, history, favorite,
   responsive, mouse, and accessibility behavior are explicit.
 - Typed job versus PTY execution and detach/reattach semantics are explicit.
 - Capability correlation and stale-authority rejection are explicit.
 - Shell operators and destructive/unsafe behavior are classified and rejected
   or separately confirmed without a shell command path.
 - Component ownership and persistence boundaries are documented.

## Verification

```bash
./scripts/verify-ui-spec.sh
./scripts/verify-roadmap.sh
./scripts/check-docs.sh
```
