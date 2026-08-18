# Current Task

## Task

**ID:** UI-LITERAL-LIVE-001
**Title:** Validate literal workbench with live Poky
**Status:** IN_PROGRESS

## Objective

Validate that the release client renders the literal workbench with real Poky
layers and recipes, clean terminal ownership, and operational F2/F10 routes.

## Dependencies

- `UI-LITERAL-UX-001` — DONE

## Relevant files

- `scripts/test-live-workbench.sh`
- `crates/yoctui-cli/src/main.rs`
- `README.md`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Live metadata includes Poky MACHINE, DISTRO, configured layers, and recipes.
- F2 enters the canonical Tasks shell and exposes the mixed project Navigator.
- F10 opens a menu containing Choose theme; applying a theme persists.
- No bridge stderr or startup notice corrupts the alternate screen.
- Workspace tests, Clippy, Python bridge tests, and roadmap checks pass.

## Verification

```bash
./scripts/test-live-workbench.sh $HOME/src/poky/build
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 -m pytest bridge/tests
./scripts/verify-roadmap.sh
```
