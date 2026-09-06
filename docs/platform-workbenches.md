# Kernel and firmware workbenches

The **Kernel** destination resolves `virtual/kernel` through the connected
BitBake backend. It reads the provider-scoped `FILE`, `S`, `B`, and `WORKDIR`
values and combines those directories with `DEPLOY_DIR_IMAGE`. Yoctui does not
guess a source or build directory from path names.

The Configuration view lists discovered `.config` files. The Device trees
view lists `.dts`, `.dtsi`, `.dtb`, and `.dtbo` files. `Enter` or `e` opens a
text file in the in-app explorer/editor, and `o` explores its authoritative
root. Binary blobs are never interpreted as text.

`m` opens `bitbake virtual/kernel -c menuconfig` in a daemon-owned persistent
PTY after the provider reports `do_menuconfig` and the user confirms the
terminal launch. This preserves the complete ncurses interaction, terminal
resize, detach, reconnect, and writer-lease behavior of other embedded
terminals.

When a canonical `dtc` executable is available, `c` compiles a selected DTS
to a sibling `NAME.yoctui.dtb`, and `d` decompiles a selected DTB or DTBO to a
sibling `NAME.yoctui.dts`. Both operations show their exact argv in the normal
terminal-launch confirmation. Existing outputs are never overwritten.

Scanning is read-only and bounded to 4,096 matching files, 16,384 directories,
and depth 32. Directory symlinks and `.git` trees are not traversed. The UI
reports scan limits and missing BitBake variables explicitly.
