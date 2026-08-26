# Current Task

## Task

**ID:** UX-INTERNAL-LOG-001
**Title:** Add a separate Yoctui self-diagnostic log view
**Status:** NOT_STARTED

## Objective

Add a bounded self-diagnostic log view for Yoctui tracing while keeping internal
diagnostics and BitBake domain logs as separate typed authorities.

## Dependencies

- `UX-LOGS-001` — DONE
- `UX-LICENSE-001` — DONE

## Relevant files

- typed bounded internal diagnostic state and tracing adapter
- internal-log filters, retention, navigation, and export
- self-diagnostic workspace or focused view and Inspector details
- admitted dependency evidence and generated notices/SBOM
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- The tui-logger candidate is evaluated against the existing license and
  dependency-admission policy before any graph change.
- Internal tracing diagnostics never enter the BitBake domain-log authority.
- Internal retention, filters, selection, loss counters, and export remain
  bounded and typed.
- Empty, filtered-empty, high-volume, Unicode, narrow, and no-color states are
  honest and panic-free.

## Verification

```bash
cargo test -p yoctui-model ux_internal_log
cargo test -p yoctui-ui ux_internal_log
cargo deny check
```
