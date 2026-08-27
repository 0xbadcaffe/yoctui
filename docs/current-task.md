# Current Task

## Task

**ID:** UX-PERF-001
**Title:** Profile and bound the expanded workbench
**Status:** NOT_STARTED

## Objective

Prove that the expanded workbench remains bounded and responsive under the
largest supported menus, rootfs inventories, graphs, editors, logs, and PTYs.

## Dependencies

- `UX-RESPONSIVE-001` — DONE

## Definition of done

- Menu-heavy, large-rootfs, large-dependency-graph, large-editor, large-log, and
  dense-terminal fixtures remain within documented model and viewport bounds.
- Production rendering retains the 10 ms/frame ceiling at supported sizes with
  measurable deterministic evidence, not a warmed-cache assumption.
- Allocation and state growth are bounded before considering any new cache;
  selection and semantic output remain correct at each large-input limit.
- Workbench and existing next-generation performance suites pass, and the
  flamegraph/profile evidence is regenerated from the current binary.

## Verification

```bash
./scripts/test-next-generation-ui-performance.sh
./scripts/test-workbench-ux-performance.sh
./scripts/test-flamegraph.sh
```
