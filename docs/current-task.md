# Current Task

**ID:** MENUCONFIG-NCURSES-001
**Title:** Preserve native menuconfig presentation in Terminal Sessions
**Status:** DONE

Give selected kernel and U-Boot menuconfig PTYs the full width remaining beside
Navigator, omit the passive Inspector, propagate the rendered terminal size to
the daemon-owned PTY, and guarantee a color-capable terminal environment. Make
the deterministic gallery fixtures exercise styled terminal cells matching the
native ncurses composition. Verify resize authority, keyboard operation,
responsive layout, both menuconfig images, and the full release baseline.

Version 0.1.109 preserves native ncurses presentation for kernel and U-Boot
menuconfig. The selected menuconfig session receives the Inspector width and
omits the prefix-help rail, writer-owned PTYs follow the exact visible split
pane dimensions, and missing or unusable `TERM` values normalize to
`xterm-256color`. Both deterministic gallery images now contain styled typed
terminal cells. Workspace tests, strict Clippy, bridge tests, docs, release
policies, and deterministic raster checks pass.
