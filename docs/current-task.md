# Current Task

## Task

**ID:** RAW-NAV-001
**Title:** Add Raw Mode Navigator destination
**Status:** IN_PROGRESS

## Objective

Add Raw Mode as one first-class inspectable workspace destination throughout
the shell's typed screen, compatibility, Navigator, command palette, help,
footer, and responsive focus-routing surfaces.

## Dependencies

- `RAW-MODEL-001` — DONE
- `UI-SHELL-001` — DONE

## Relevant files

- `crates/yoctui-model/src/lib.rs`
- `crates/yoctui-model/src/workspace_compatibility.rs`
- `crates/yoctui-app/src/lib.rs`
- `crates/yoctui-ui/src/lib.rs`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- `Screen`, labels, workspace destinations, compatibility projection, palette,
  and typed routing expose exactly one `Raw Mode` identity.
- Raw Mode appears in the Navigator `TOOLS` group at the specified position
  without changing unrelated stable destinations or function-key routes.
- Enter/palette activation opens Raw Mode and normal Navigator/Workspace/
  Inspector focus cycling remains shared shell behavior.
- Help and contextual footer expose the authoritative Raw navigation/search/
  favorite/history bindings without claiming execution availability.
- Wide, medium, narrow, and below-minimum terminal routing remains safe and
  selection-preserving; no Raw browser layout is invented in this task.
- Model, app, and TestBackend tests cover every entry route, compatibility
  inspection, labels/help/footer, focus routing, and responsive shell state.

## Verification

```bash
cargo test -p yoctui-model raw_navigation
cargo test -p yoctui-app raw_navigation
cargo test -p yoctui-ui raw_navigation
cargo clippy -p yoctui-model -p yoctui-app -p yoctui-ui --all-targets --all-features -- -D warnings
./scripts/verify-roadmap.sh
```
