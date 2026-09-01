# Current Task

## Task

**ID:** UX-DOT-METERS-001
**Title:** Unify telemetry and progress with hot-dot meters
**Status:** DONE

## Objective

Telemetry and every determinate or indeterminate progress surface must use one
compact square-dot visual language while preserving exact textual authority.

## Dependencies

- ROOTFS-PKGDATA-VIEWPORT-001 — DONE

## Definition of done

- CPU, RAM, and build-filesystem meters use segmented square-dot tracks with
  threshold heat colors.
- Overall build, task, and reusable semantic progress use the same vocabulary.
- Running and waiting markers contain no circular spinner glyphs.
- Unicode, ASCII, no-color, and reduced-motion states preserve exact text.
- Six exact concept captures and deterministic PNGs come from the production
  renderer.
- Version 0.1.11 is installed and repository completion gates pass.

## Verification

```bash
cargo test -p yoctui-ui
./scripts/verify-m22-concept-parity.sh
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
./scripts/verify-completion.sh
```

The meters are an original implementation using Ratatui cells and the existing
semantic theme roles. They take visual inspiration from segmented system
monitors without copying btop source code, assets, layout, or branding.
