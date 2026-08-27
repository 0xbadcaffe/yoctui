# Current Task

## Task

**ID:** UX-CONCEPT-ERRORS-001
**Title:** Compose the complete failed-build concept workflow
**Status:** IN_PROGRESS

## Objective

Compose the failed-build concept through the production renderer: failed
summary, structured diagnostics, correlated paused log with match/loss state,
textual warning/error filters, and recovery actions must be visible together.

## Dependencies

- UX-CONCEPT-ACCEPTANCE-001 — DONE

## Definition of done

- The failed lifecycle and selected diagnostic agree.
- The correlated log is paused and exposes match and retention-loss state.
- Warning/error filter checkboxes have accessible textual meaning.
- Recovery actions expose exact availability and confirmation requirements.
- Focused model and production-renderer tests pass.

## Verification

```bash
cargo test -p yoctui-ui concept_failed_build
cargo test -p yoctui-model concept_failed_build
```
