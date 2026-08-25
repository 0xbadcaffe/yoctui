# Current Task

## Task

**ID:** M13-UI-001
**Title:** Complete next-generation Yoctui TUI
**Status:** IN_PROGRESS

## Objective

Close the parent next-generation UI milestone only after the redesigned typed
workbench, real-Poky evidence, live documentation, rendering-module cleanup,
and the full non-weakened repository completion gate all pass together.

## Dependencies

- `LIVE-UI-POKY-001` — DONE
- `README-UI-001` — DONE
- `UI-REGRESSION-001` — DONE
- `UI-CLEANUP-001` — DONE
- All other registered M19 UI children — DONE

## Relevant files

- `scripts/verify-next-generation-ui.sh`
- `scripts/verify-completion.sh`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Pass the complete next-generation UI verification script.
- Pass the full repository completion gate without weakening checks.
- Confirm all required tasks and milestone parents are terminally complete.
- Commit the terminal governance state with no task left active.

## Verification

```bash
./scripts/verify-next-generation-ui.sh
./scripts/verify-completion.sh
```
