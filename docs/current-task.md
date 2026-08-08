# Current Task

## Task

**ID:** CI-RUFF-001
**Title:** Pin bridge Ruff policy against evolving defaults
**Status:** DONE

## Objective

Final completed task: make the bridge's deliberate adapter-boundary lint policy
explicit across Ruff releases.

## Verification

```bash
ruff check bridge
ruff format --check bridge
python3 -m pytest bridge/tests
```

## Definition of done

- Bridge lint and tests pass with the configured Ruff policy.

## Next task

## Terminal handoff

All registry tasks are complete.
