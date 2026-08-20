# Current Task

## Task

**ID:** PERF-UI-002
**Title:** Add render caching where justified
**Status:** IN_PROGRESS

## Objective

Remove the measured large-recipe per-frame hot path by bounding row projection
to the visible viewport and caching only model-owned filtered indices or
normalized query state where measurement justifies it. Preserve exact
selection/query invalidation and prove that no stale UI state can appear.

## Dependencies

- `PERF-UI-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-cli/benches/workbench_profile.rs`
- `scripts/test-next-generation-ui-performance.sh`
- `docs/profiling.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- Recipe and layer tables construct/format only the bounded visible row window,
  not the complete filtered dataset, on each frame.
- Filtered indices and normalized queries, if cached in the model, invalidate
  on query, workspace inventory, selection, refresh, and error transitions.
- Selection identity and scroll position remain stable and bounded after every
  invalidation; empty and no-match states cannot retain stale rows.
- Static labels or sparkline points are cached only if the measured matrix
  still identifies them as material after viewport bounding.
- `render_cache` model tests exercise cache reuse and every invalidation path.
- The five-scenario matrix shows a material large-metadata improvement without
  regressing idle, active-build, log-heavy, or telemetry thresholds.

## Verification

```bash
./scripts/test-next-generation-ui-performance.sh
cargo test -p yoctui-model render_cache
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
