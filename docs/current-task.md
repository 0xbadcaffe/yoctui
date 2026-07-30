# Current task

## Active task

**ID:** SEC-RENDER-001
**Title:** Render responsive Security workspace

## Objective

Render the complete typed Security workspace and dialogs at every supported
responsive breakpoint without parsing backend text in widgets.

## Required work

1. Inspect the shared shell, existing destination renderers, Security model,
   and authoritative Security section of `docs/ui-spec.md` before editing.
2. Add Security destination rendering for CVE and SBOM views using only typed
   capability, inventory, finding, document/component, session, and limitation
   state.
3. Render explicit not-inspected/loading/unavailable, not-loaded/loading,
   available-empty, available, partial, failed, cancelled, timed-out, and lost
   states without showing stale rows as current.
4. Render exact CVE status and source identity, searchable/filterable finding
   detail, mapping metadata, advisory/provider availability, and limitations.
5. Render exact SPDX document/archive identity, schema summary, creators,
   checksums/counts, component drill state, unsupported/archive limitations,
   and related-action availability.
6. Render operation, import, and cancellation dialogs as focus-trapping,
   indexed, exact previews with disabled reasons; no widget may infer task,
   report, path, status, or command authority.
7. Render bounded mapper/build session lifecycle and retained typed output,
   plus the exact responsive Security footer and visible scope/capability
   context.
8. Add Ratatui `TestBackend` coverage for wide, medium, 80x24/narrow, long
   fields, every explicit inventory/session outcome, all dialogs, every theme,
   and no-color semantics. Narrow terminals must never panic.
9. Update `docs/ui-spec.md` in the same commit only if an intentional behavior
   change is required; otherwise implement its existing contract exactly.

## Definition of done

- The Security destination and all dialogs render only typed model state.
- Wide, medium, narrow/80x24, themes, no-color, long data, explicit lifecycle
  states, selections, drill state, limitations, and footer hints are covered.
- Focus, responsive degradation, and stale/empty/failure presentation match
  `docs/ui-spec.md` and never panic.
- Focused UI and baseline checks pass.

## Verification

```bash
cargo test -p yoctui-ui security_workflow
cargo fmt --all --check
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```

## Next task

Select the next eligible highest-priority incomplete task from
`docs/task-registry.toml`.
