# Current Task

## Task

**ID:** UI-LIVE-DISCOVERY-001
**Title:** Make theme and pane focus directly discoverable
**Status:** IN_PROGRESS

## Objective

Provide a direct command-palette route to the named theme picker and make focus
movement self-explanatory by naming the current, next, and previous pane.

## Dependencies

- `UI-LIVE-STARTUP-001` — DONE

## Relevant files

- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-cli/src/main.rs`
- `docs/ui-spec.md`
- `docs/task-registry.toml`
- `docs/implementation-status.md`

## Definition of done

- `Choose theme` is available through the command palette.
- The picker and Settings use one typed theme workflow.
- Theme selection enables color unless `--no-color` is authoritative.
- The command rail names active, next, and previous focus destinations.
- Model, app, UI, formatting, and roadmap checks pass.

## Verification

```bash
cargo test -p yoctui-model theme
cargo test -p yoctui-ui focus
cargo test -p yoctui-app focus
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
