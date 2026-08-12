# Current Task

## Task

**ID:** PTY-MENUCONFIG-001
**Title:** Support interactive menuconfig and devshell PTYs
**Status:** IN_PROGRESS

## Objective

Add typed daemon-PTY requests for `bitbake -c menuconfig <recipe>`, kernel and
U-Boot menuconfig, `devshell`, and other explicitly allowlisted interactive
BitBake tasks. Validate authoritative recipe/task identity and exact argv,
reuse the verified build environment, preview before launch, and never suspend
or hand terminal ownership to the main client.

## Verification

```bash
cargo test -p yoctui pty_menuconfig
```
