# Current Task

## Task

**ID:** UX-LICENSE-001
**Title:** Establish third-party widget license and supply-chain gate
**Status:** NOT_STARTED

## Objective

Create the reusable dependency-admission gate required before any showcased
third-party widget is added to Yoctui.

## Dependencies

- `UX-SPEC-001` — DONE

## Relevant files

- `Cargo.toml`
- `Cargo.lock`
- `deny.toml`
- `docs/workbench-ux-roadmap.md`
- third-party notice and SBOM configuration
- `scripts/verify-third-party-notices.sh`
- `scripts/verify-widget-dependencies.sh`

## Definition of done

- Refresh exact candidate crate versions, SPDX licenses, sources/checksums,
  MSRVs, Ratatui compatibility, enabled features, and transitive dependencies.
- Reject any crate or feature set that violates repository policy.
- Generate and verify complete third-party notices and an auditable SBOM.
- Prove the selected dependency graph builds from the lockfile without network.
- Document explicit adopt, adapt, defer, or reject decisions without importing
  showcase application code or assets.

## Verification

```bash
cargo deny check
./scripts/verify-third-party-notices.sh
./scripts/verify-widget-dependencies.sh
./scripts/verify-roadmap.sh
```

Do not add a widget dependency in this task. This task creates the gate that
later widget-specific tasks must pass immediately before adoption.
