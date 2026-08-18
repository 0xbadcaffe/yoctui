# Current Task

## Task

**ID:** UI-VISION-SHELL-001
**Title:** Implement compact workbench shell chrome
**Status:** IN_PROGRESS

## Objective

Replace the oversized telemetry header and prose footer with the compact
one-line project/status header, dense panel chrome, and contextual command rail
defined by the approved workbench visual specification.

## Dependencies

- `UI-VISION-SPEC-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Header occupies one bordered row and prioritizes project plus daemon/BitBake state.
- Footer renders a compact contextual key/action rail with a right-aligned clock.
- Focus, themes, no-color mode, and all supported terminal widths remain safe.
- Deterministic workbench shell tests pass.

## Verification

```bash
cargo test -p yoctui-ui workbench_shell
cargo test -p yoctui-ui theme
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
