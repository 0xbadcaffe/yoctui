# Kernel and firmware

## Sources and configuration

Kernel resolves virtual/kernel; U-Boot / BIOS resolves the configured boot provider.
FILE/S/B/WORKDIR/DEPLOY_DIR_IMAGE metadata supplies roots; paths are not guessed.
Configuration lists .config; Device trees lists DTS/DTSI/DTB/DTBO.
Enter/`e` edits text, `o` explores the root, `r` refreshes; binary blobs are not text.
Scans stop at 4,096 matching files, 16,384 directories, depth 32; no directory symlinks.

`m` reviews the provider's supported menuconfig task in a persistent PTY.
With dtc available, `c` compiles to NAME.yoctui.dtb and `d` decompiles to
NAME.yoctui.dts. Review covers symbols, sorting, padding, and reserve entries;
existing outputs are never overwritten. Use [terminal writer controls](terminal-sessions.md).

## Kernel debugging

Kernel `3`/`b` opens Debugging independently of a completed provider scan.
Arrows select; Enter opens the form; `r` refreshes tools. Tab changes fields,
Ctrl+U clears, PgUp/PgDn scrolls guidance/review. Final confirmation selects an
embedded/detached terminal and launches the exact argv.

Defaults use existing artifacts from the selected build/machine and preserve
user edits. Missing/ambiguous values stay empty; use matching uncompressed vmlinux
with debug symbols (possibly under boot/.debug), not a stripped boot image.

| Route | Prerequisite / scope |
| --- | --- |
| GDB TCP | Already configured QEMU/KGDB stub, architecture-capable GDB, matching symbols. |
| GDB executable/core | Matching userspace files; distinct from kernel vmcore. |
| strace / perf top | Explicit PID and permissions. |
| trace-cmd / ftrace | Existing trace.dat / tracefs snapshot; capture/configuration is manual. |
| dmesg / dynamic debug / kmemleak | Existing logs/callsites/report; no clear or scan. |
| bpftrace / LTTng | One validated syscall-entry tracepoint / list existing kernel events. |
| crash | Matching vmlinux/vmcore; kdump collection is manual. |

SSH runtime tools need explicit host/user/port and remote installation/permissions;
Host mode operates on the Yoctui host. Native diagnostics retain target failures.
GDB init/auto-load/debuginfod and crashrc are disabled. No automatic sudo, installs,
kernel changes, tracing configuration, or reboot. Debuggers can pause execution.

## Serial KGDB for an already configured board

Choose KGDB → GDB · serial board. Supply matching ELF/DWARF vmlinux, exact .config,
real host tty (e.g. /dev/ttyUSB0), baud, and board UART (e.g. ttyAMA0). Close other
console clients; type `yes` only when the target is already stopped and ready.

Preparation checks bounded files/serial metadata without opening the port.
Requires CONFIG_KGDB=y, CONFIG_KGDB_SERIAL_CONSOLE=y, CONFIG_DEBUG_INFO=y;
module-only kgdboc is unsupported. Review shows manual kgdboc=UART,BAUD nokaslr
setup; optional kgdbwait follows transport configuration. Nothing changes boot args.

Final confirmation revalidates and launches GDB. Native bt/break/continue/detach/quit
apply; Ctrl+C cannot reliably stop a running kgdboc board. Manual re-entry may need
SysRq-G. Killing the host client does not guarantee resume. Yoctui sends no SysRq,
break, flash/reset/reboot. Matching hardware validation remains separate from PTY fixtures.
See [Linux KGDB](https://docs.kernel.org/process/debugging/kgdb.html).
