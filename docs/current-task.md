# Current Task

## Task

**ID:** PTY-CONTEXT-001
**Title:** Open PTYs in typed Yocto contexts
**Status:** IN_PROGRESS

## Objective

Add typed terminal creation actions for build directory, source tree, selected
layer, selected recipe source, Devtool workspace, verified SDK environment and
image/deploy directory contexts. Canonicalize and authorize paths against the
current workspace, construct exact shell identity from trusted configuration,
and never execute project-profile shell strings.

## Verification

```bash
cargo test -p yoctui-app pty_context
```
