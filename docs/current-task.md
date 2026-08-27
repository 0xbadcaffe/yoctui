# Current Task

## Task

**ID:** UX-CONCEPT-ROOTFS-001
**Title:** Compose the canonical Rootfs exploration concept
**Status:** IN_PROGRESS

## Objective

Compose the Rootfs exploration concept through the production renderer: chart,
exact package table, accessible batch selection, limitations, and filesystem
drill-down must remain simultaneously visible at the canonical width.

## Dependencies

- UX-CONCEPT-ACCEPTANCE-001 — DONE

## Definition of done

- The Rootfs chart and exact package table coexist at 160×50.
- Selection exposes accessible checkbox semantics and exact package identity.
- Limitations and unavailable states remain explicit.
- Filesystem drill-down is separately labelled and visible in the same scene.
- Focused production-renderer and input-routing tests pass.

## Verification

```bash
cargo test -p yoctui-ui concept_rootfs_composition
cargo test -p yoctui-app ux_rootfs
```
