# Current Task

## Task

**ID:** UX-CONCEPT-VALIDATION-001
**Title:** Validate the six visual concepts through the production Yoctui renderer
**Status:** NOT_STARTED

## Objective

Create an executable acceptance baseline for all six M21 concept scenes using
typed fixtures and Yoctui's production Ratatui renderer.

## Dependencies

- `UX-SPEC-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-ui/tests/golden/`
- `docs/design/m21/concepts/manifest.toml`
- `docs/ui-spec.md`
- `docs/workbench-ux-roadmap.md`
- `scripts/update-m21-concept-screen-goldens.sh`
- `scripts/verify-m21-concept-screens.sh`

## Definition of done

- Idle Dashboard, active Tasks, failed Errors, Images/rootfs, editor/menu, and
  terminal-session fixtures render through production `render_at` at `160x50`.
- Every scenario checks reviewed semantic anchors and serializes every Ratatui
  cell symbol and style into an explicit-update golden.
- The concept manifest maps each scene to its implementation tasks and names
  every remaining concept gap without weakening the passing current baseline.
- The verifier rejects missing/stale goldens, unregistered gap tasks, completed
  tasks that still claim gaps, and accidental use of generated PNGs as exact
  pixel authority.
- Documentation distinguishes AI concept direction, deterministic real-Yoctui
  acceptance, and live PTY evidence.

## Verification

```bash
cargo test -p yoctui-ui concept_screen_contracts
./scripts/verify-m21-concept-screens.sh
./scripts/verify-m21-concept-pack.py
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
