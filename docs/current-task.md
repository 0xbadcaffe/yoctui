# Current Task

## Task

**ID:** YOCTO-LOGGER-ADAPTER-001
**Title:** Integrate tui-logger presentation with authoritative Yocto log records
**Status:** IN_PROGRESS

## Objective

Use the upstream log widget for Yocto log/output presentation without replacing
authoritative retention, correlation, filters, search, bookmarks, export, or
critical-event handling. Keep diagnostics and interactive PTYs separate.
Bound any upstream projection buffer to the visible viewport.

## Dependencies

- CONSOLE-TERM-002 — DONE (v0.1.59)

## Definition of done

- Logs and embedded Yocto output use tested tui-logger presentation.
- Repeated rendering cannot duplicate records or leak between panes/apps.
- Dependencies, MIT notices, compiler compatibility, and SBOM are audited.
- Production fixtures and rendering measurements validate the adapter.

## Verification

```bash
cargo test --workspace log
./scripts/verify-widget-dependencies.sh
cargo deny check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```

Next: LOG-CONSOLE-IMAGE-001.
