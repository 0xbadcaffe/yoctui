# Current Task

## Task

**ID:** SHELL-MODEL-001
**Title:** Model embedded shell sessions and input ownership
**Status:** NOT_STARTED

## Objective

Define stable shell session IDs, lifecycle, cwd/environment identity,
foreground/copy/search modes, bounded scrollback, exit status, and exclusive
input ownership with an emergency escape chord.

## Verification

```bash
cargo test -p yoctui-model embedded_shell
cargo test -p yoctui-app embedded_shell
```

## Definition of done

- Shell session state transitions and input ownership are typed and tested.

## Next task

After completion, select `SHELL-PTY-001`.
