# Current Task

## Task

**ID:** UI-STARTUP-LIVE-001
**Title:** Validate installed startup and theme selection on live Poky
**Status:** IN_PROGRESS

## Objective

Exercise the real Poky workbench through the shell-installed release binary,
reject diagnostics outside the terminal frame, and prove that command-palette
theme selection changes and persists the operator preference.

## Dependencies

- `UI-STARTUP-STDERR-001` — DONE

## Relevant files

- `scripts/test-live-workbench.sh`
- `README.md`
- `~/.cargo/bin/yoctui`

## Definition of done

- The live PTY contains no BitBake startup output before alternate-screen entry.
- `Ctrl+P` → `Choose theme` opens the picker and persists a changed theme.
- The locally built release binary is installed at the shell-resolved path.
- Development documentation explains how to refresh a same-version install.

## Verification

```bash
./scripts/test-live-workbench.sh $HOME/src/poky/build
cargo build --release -p yoctui
cmp target/release/yoctui $HOME/.cargo/bin/yoctui
./scripts/verify-roadmap.sh
```
