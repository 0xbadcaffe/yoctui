# Current Task

## Task

**ID:** UI-LITERAL-SHELL-001
**Title:** Match the reference workbench shell
**Status:** IN_PROGRESS

## Objective

Match the approved header, pane boundaries, compact borders, palette hierarchy,
and stable F-key command rail at the canonical `160x48` size.

## Dependencies

- `UI-LITERAL-HARNESS-001` — DONE

## Relevant files

- `crates/yoctui-ui/src/lib.rs`
- `crates/yoctui-ui/tests/golden/literal-reference-160x48.cells`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`

## Definition of done

- Canonical header/body/footer rectangles match the specified coordinates.
- Tasks uses 26/89/45-column body panes at 160 columns.
- The default palette preserves near-black surfaces, blue selection, amber
  navigation, lime progress/success, cyan information, and red failure.
- The two-row footer shows the stable F1 through F10 reference command rail.
- Existing medium, narrow, and too-small rendering remains safe.

## Verification

```bash
cargo test -p yoctui-ui literal_shell
cargo test -p yoctui-ui workbench_shell
cargo fmt --all --check
./scripts/verify-roadmap.sh
```
