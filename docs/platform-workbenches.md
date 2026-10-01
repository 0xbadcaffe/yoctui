# Kernel and firmware workbenches

The **Kernel** destination resolves `virtual/kernel` through the connected
BitBake backend. It reads the provider-scoped `FILE`, `S`, `B`, and `WORKDIR`
values and combines those directories with `DEPLOY_DIR_IMAGE`. Yoctui does not
guess a source or build directory from path names.

The separate **U-Boot / BIOS** destination resolves the active image's boot
firmware. It checks image-scoped and global
`PREFERRED_PROVIDER_virtual/bootloader`, `VIRTUAL-RUNTIME_bootloader`,
`UBOOT_MACHINE`, and `EFI_PROVIDER` values, then tries the virtual bootloader
and configured U-Boot, EDK2/OVMF, SeaBIOS, coreboot, GRUB EFI, or systemd-boot
recipes reported by the workspace. The first recipe with authoritative
metadata becomes the target. Provider identity labels the workspace as
U-Boot, BIOS / UEFI, or the unresolved-safe U-Boot / BIOS fallback.

The Configuration view lists discovered `.config` files. The Device trees
view lists `.dts`, `.dtsi`, `.dtb`, and `.dtbo` files. `Enter` or `e` opens a
text file in the in-app explorer/editor, and `o` explores its authoritative
root. DTS and DTSI use Device Tree syntax highlighting for directives, labels,
nodes, properties, values and comments while retaining the shared editor's
search, undo/redo, diff, validation and guarded save behavior. Binary blobs are
never interpreted as text.

`m` opens `bitbake virtual/kernel -c menuconfig` for Kernel or
`bitbake DETECTED_PROVIDER -c menuconfig` for boot firmware in a daemon-owned
persistent PTY after that exact provider reports `do_menuconfig` and the user
confirms the terminal launch. BIOS/UEFI providers without a Kconfig task remain
browsable and show menuconfig as unavailable. The PTY preserves the complete
ncurses interaction, terminal resize, detach, reconnect, and writer-lease
behavior of other embedded terminals.

When a canonical `dtc` executable is available, `c` opens a typed compile form
for symbol generation (`-@`), stable sorting (`-s`), output padding (`-p`) and
reserve-map entries (`-R`). Enter advances to the normal terminal-launch
confirmation, which shows the exact argument vector before the compiler runs
in a daemon-owned utility PTY. The derived output is a sibling
`NAME.yoctui.dtb`. `d` decompiles a selected DTB or DTBO to a sibling
`NAME.yoctui.dts` through the same preview and PTY boundary. Existing outputs
are never overwritten; press `r` after completion to refresh the inventory.

Scanning is read-only and bounded to 4,096 matching files, 16,384 directories,
and depth 32. Directory symlinks and `.git` trees are not traversed. The UI
reports scan limits and missing BitBake variables explicitly.

## Kernel debugging

With Kernel Workspace focus, `3` or `b` opens Debugging; Tab cycles all three
Kernel views. U-Boot remains two-tab. Arrows/paging select a technique, Enter
opens its form or guide, and `r` refreshes host executable discovery. This tab
does not need completed kernel artifacts or a successful provider scan.

Forms use Tab/Up/Down for fields, typed text/Backspace/Ctrl+U for values,
Left/Right/Space for Host/SSH scope, PageUp/PageDown for guidance, Enter for exact
launch review, and Esc to cancel. Review shows each argument independently;
PageUp/PageDown scroll long arguments. Up/Down chooses embedded or detached
terminal; only the subsequent Enter starts the process. Existing PTY writer,
resize, detach/reattach and confirmed termination behavior is reused.

Runtime tools default to SSH with a required explicit host/user/port. Local
OpenSSH presence does not establish remote-tool installation or privilege;
target errors stay in terminal output. No sudo or automatic package install is
used. Host mode deliberately operates on the Yoctui host, not the image/board.
SSH uses normal host-key checking and account authentication. Hosts/endpoints
accept DNS/IPv4, not shell commands, SSH URIs or serial/IPv6 endpoint syntax.

The launchable routes are:

- GDB TCP remote client for an already configured QEMU/KGDB stub, with matching
  uncompressed vmlinux symbols and architecture-capable host GDB. Discovery
  prefers gdb-multiarch, then gdb; configure a suitable debugger on PATH. No QEMU
  boot or stub setup is performed. [Kernel KGDB prerequisites](https://docs.kernel.org/process/debugging/kgdb.html).
- GDB userspace executable/core analysis, distinct from kernel vmcore analysis.
  Startup init files, auto-loaded scripts and debuginfod downloads/prompts are
  disabled; supply local matching symbols. Remote mode also
  prevents accidental local inferior startup. [GDB auto-loading controls](https://sourceware.org/gdb/current/onlinedocs/gdb/Auto_002dloading.html).
  [Debuginfod controls](https://www.sourceware.org/gdb/download/onlinedocs/gdb.html/Debuginfod-Settings.html).
- strace attaches to an explicit PID with child following and syscall timing;
  it traces userspace/kernel interactions, not kernel source code. [strace](https://strace.io/).
- perf top samples an explicit PID; trace-cmd reports an existing local trace.dat
  with automatic plugins disabled. Capture/configuration remains manual.
  [trace-cmd report](https://www.trace-cmd.org/Documentation/trace-cmd/trace-cmd-report.1.html).
- ftrace reads the existing tracefs snapshot without consuming trace_pipe,
  clearing buffers, mounting filesystems or enabling tracers. [ftrace](https://docs.kernel.org/trace/ftrace.html).
- dmesg follows kernel messages without clearing them; dynamic debug lists
  callsites; kmemleak reads an existing report without triggering a scan.
  [Dynamic debug](https://docs.kernel.org/admin-guide/dynamic-debug-howto.html),
  [kmemleak](https://docs.kernel.org/dev-tools/kmemleak.html).
- bpftrace counts one validated syscall-entry tracepoint by process name; Ctrl+C
  ends instrumentation and prints counts. LTTng lists kernel events without
  creating a recording session. [bpftrace language](https://bpftrace.org/docs/release_024/language),
  [LTTng list](https://lttng.org/man/1/lttng-list/v2.15/).
- crash analyzes explicit matching vmlinux/vmcore files with crashrc startup
  files disabled; collecting kdump is separate and manual. [crash utility](https://crash-utility.github.io/).

KGDB/KDB setup, KASAN/KCSAN/UBSAN, lockdep/hung tasks and SysRq/kdump entries are
guidance only. Yoctui does not change kernel config, mount debugfs/tracefs,
write tracing controls, trigger a panic or reboot a target. A debugger can pause
execution; profiling/tracing changes timing and may expose sensitive data.
Required options, matching architecture/symbols and permissions remain the
operator's responsibility. Missing tools block launch with explicit guidance;
unsupported/missing target features fail visibly rather than claiming success.
