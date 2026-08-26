# Current Task

## Task

**ID:** UX-ROOTFS-MODEL-001
**Title:** Define typed root filesystem composition state
**Status:** NOT_STARTED

## Objective

Define image-correlated installed-package and filesystem-tree composition
authority with exact totals, bounded grouping/drilldown, stable selection, and
honest lifecycle/limitation states.

## Dependencies

- `UX-SPEC-001` — DONE
- `IMAGES-001` — DONE
- `PKG-001` — DONE

## Relevant files

- rootfs composition model and protocol records
- image-artifact correlation and generation identity
- installed-package and filesystem-tree authority states
- grouping, totals, percentages, drilldown, selection, and limitations
- model/protocol/app normalization tests

## Definition of done

- Installed-package and filesystem-tree authorities remain separate and are
  correlated to an exact image artifact and request generation.
- Exact bytes, counts, totals, percentages, grouped `Other`, and drilldown rows
  are deterministic, bounded, and overflow-safe.
- Not-loaded, loading, available-empty, available, partial, unavailable, and
  failed states preserve limitations without presenting stale data as current.
- Selection survives replacement by stable identity and falls back explicitly.

## Verification

```bash
cargo test -p yoctui-model ux_rootfs
cargo test -p yoctui-protocol ux_rootfs
cargo test -p yoctui-app ux_rootfs
```
