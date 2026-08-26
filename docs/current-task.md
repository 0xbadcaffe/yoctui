# Current Task

## Task

**ID:** UX-DEPENDENCY-GRAPH-001
**Title:** Add navigable dependency topology visualization
**Status:** NOT_STARTED

## Objective

Add a bounded, navigable dependency-topology view over authoritative recipe and
package dependency data with stable selection and a complete text fallback.

## Dependencies

- `UX-LIST-TREE-001` — DONE
- `UX-LICENSE-001` — DONE

## Relevant files

- dependency graph normalization and workspace reducer state
- dependency workspace keyboard/mouse routes
- graph/tree/table render adapters and responsive fallbacks
- `tui-nodes` candidate decision and compliance evidence
- `docs/ui-spec.md`
- `docs/architecture.md`

## Definition of done

- Nodes and edges retain authoritative stable identities, direction, cycles,
  missing/partial authority, and hard count/depth bounds.
- Keyboard and mouse navigation, expand/collapse, path inspection, reverse view,
  filtering, and exact provider/log jumps are typed and selection-stable.
- Wide topology, medium tree, narrow table, ASCII, no-color, and screen-reader
  text projections preserve the same relationships and numeric position.
- The `tui-nodes` spike is either admitted without a second state authority or
  rejected with tested custom-renderer parity evidence.

## Verification

```bash
cargo test -p yoctui-model ux_dependency_graph
cargo test -p yoctui-app ux_dependency_graph
cargo test -p yoctui-ui ux_dependency_graph
cargo deny check
```
