# Current Task

## Task

**ID:** PTY-UI-TEST-001
**Title:** Test embedded terminal rendering
**Status:** IN_PROGRESS

## Objective

Exercise embedded terminal behavior through real PTY fixtures and prove that
session rendering, resize, lifecycle, focus transfer, escape-chord handling,
and bounded scrollback do not corrupt the surrounding workbench.

## Dependencies

- `INPUT-TEST-002` — DONE
- `INPUT-TEST-003` — DONE

## Relevant files

- `crates/yoctui-e2e/`
- `crates/yoctui-shell/`
- `crates/yoctui-ui/`
- `crates/yoctui-app/`
- `scripts/test-embedded-shell.sh`
- `docs/ui-spec.md`
- `docs/implementation-status.md`
- `docs/task-registry.toml`
- `docs/current-task.md`

## Definition of done

- A real PTY fixture renders inside the selected terminal pane without altering
  the workbench header, surrounding panes, or footer.
- Resize, attach/detach, focus transfer, escape chord, and bounded scrollback
  have explicit acceptance coverage.
- Session output and lifecycle transitions remain typed and bounded.
- Focus and writer ownership rules prevent input from leaking between the
  embedded session and the workbench.

## Verification

```bash
cargo test -p yoctui-e2e next_generation_pty
./scripts/test-embedded-shell.sh
cargo fmt --all --check
./scripts/check-docs.sh
./scripts/verify-roadmap.sh
```
