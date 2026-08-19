# Current Task

## Task

**ID:** FOUNDATION-UI-001
**Title:** Specify the new visual layout contract
**Status:** IN_PROGRESS

## Objective

Define the authoritative layout and responsive behavior for the
next-generation terminal workbench before changing layout code.

## Dependencies

- `M19-GOV-001` — DONE

## Relevant files

- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Exact region hierarchy and minimum dimensions are normative.
- Wide, medium, narrow, and below-minimum behavior is explicit.
- Pane priority, Inspector collapse, footer and metric-strip behavior are
  defined.
- Responsive column hiding, focus, scroll indicators, and empty/loading/error
  states are defined.
- Unsupported concept-image elements are explicitly adapted or omitted.

## Verification

```bash
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
