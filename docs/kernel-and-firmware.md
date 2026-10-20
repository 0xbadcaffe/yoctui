# Kernel and firmware

## Sources and configuration

Kernel resolves virtual/kernel; U-Boot / BIOS resolves the configured boot provider.
FILE/S/B/WORKDIR/DEPLOY_DIR_IMAGE metadata supplies roots; paths are not guessed.
Configuration lists .config; Device trees lists DTS/DTSI/DTB/DTBO.
Enter/`e` edits text, `o` explores the root, `r` refreshes; binary blobs are not text.
Scans stop at 4,096 matching files, 16,384 directories, depth 32; no directory symlinks.

`m` reviews the provider's supported menuconfig task in a persistent PTY.
Preparation runs in the background: navigation, menus and other screens remain
available. The interactive menu appears when ncurses is ready, without switching
you away from another screen. In the kernel/U-Boot workspace, Ctrl+G returns to
Yoctui or resumes the menu; during preparation it toggles automatic opening.
Unchanged launches reuse the terminal configuration and BitBake recipe cache.
Detached menuconfig initializes the selected source/build environment too, so
an unsourced `yoctui attach` shell does not trigger a separate recipe-cache rebuild.
The first launch, changed metadata or missing dependencies can still take time;
normal configuration and recipe freshness checks remain enabled.

With dtc available, `c` compiles to NAME.yoctui.dtb and `d` decompiles to
NAME.yoctui.dts. Review covers symbols, sorting, padding, and reserve entries;
existing outputs are never overwritten. Use [terminal writer controls](terminal-sessions.md).

## Kernel debugging

Select Kernel in the navigator and press Enter to focus its workspace; `3`/`b`
then opens Debugging independently of a completed provider scan.
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
Kernel routes load symbols without enabling host execution. GDB's "no executable
specified" warning can occur in this symbols-only mode; missing/incorrect symbols
or source files are separate problems, not proof of a working debug connection.

Managed QEMU debugging maps Yocto's `/usr/src/kernel` debug prefix to retained
`tmp/work-shared/<machine>/kernel-source` when the selected qemuboot artifact and
source belong to that build. The terminal reports the mapping; sources outside
the build are not followed. Custom debug prefixes/layouts still need an explicit
GDB `set substitute-path OLD NEW` command. Always use sources matching vmlinux.
The TCP GDB route also discovers source and generated-header mappings when its
default symbols belong to the selected build/machine's retained kernel build.
The review shows the mappings. Selecting different symbols drops the old map;
custom layouts or unavailable retained sources still require manual mapping.

For a direct x86 QEMU guest, the managed-boot form offers an optional localhost
SSH port (1024–65535). Leaving it empty preserves network isolation. Setting it
adds one restricted user-mode NIC and only `127.0.0.1:PORT → guest :22`; the GDB
stub remains on its private Unix socket. The guest cannot initiate outside
connections. The selected image must already include/configure an SSH server,
credentials and the desired target diagnostic tools. No packages, account or
firewall changes happen on launch. Other architectures and flash mode remain
isolated in this workflow. The review displays the exact forward and argv.
Loopback binding is not authentication: other local users can reach that port.
Use suitable guest credentials on shared hosts; empty-root demo images are not
a production security profile.

Use GDB `continue` to boot before connecting diagnostics to `127.0.0.1`, the
chosen SSH port and the guest account. A breakpoint or Ctrl+C pauses the entire
guest, including SSH; this is not a daemon disconnect. GDB `quit` stops this
owned guest and the forward. Detaching a Yoctui client leaves the daemon-owned
debug terminal running; return through Terminal Sessions and take writer control.

The debugging list's “SSH ready” means the host SSH client exists; it does not
claim the target tool or its permissions were verified. Select an explicit PID
for strace/perf. Perf defaults to `cpu-clock`, which needs no hardware PMU;
`task-clock` is also a software counter. Only select `cycles`/`instructions` when
the physical or virtual PMU supports them. No host privilege settings are changed.

### Practical QEMU debugging loop

1. Build an image with an SSH server and the desired target tools. Retain the
   matching uncompressed vmlinux, kernel sources and resolved kernel `.config`.
2. Open Kernel → Debugging → QEMU → GDB. Check every detected artifact belongs to
   that image, select sufficient guest memory and an unused localhost SSH port
   (for example 2245), then review and launch in an embedded terminal.
3. At GDB's prompt, use `hbreak start_kernel` followed by `continue`. A real hit
   should allow `bt`, `info registers` and `list`; missing source needs a matching
   source path, not different symbols. Delete the breakpoint and `continue` to
   boot. Ctrl+C pauses the guest for further inspection.
4. Once booted, verify the guest with `ssh -p 2245 root@127.0.0.1 uname -r`, using
   the image's configured account/authentication. In the other debugging forms,
   choose SSH target, host `127.0.0.1` and that port. Choose an expendable workload
   PID for strace/perf, not PID 1 or a critical service. Host tools do not substitute
   for missing guest tools or kernel support.
5. Tracing files require tracefs/debugfs to be mounted in the guest. Enable only
   the events needed for a bounded capture there. For example, with guest
   trace-cmd installed, `trace-cmd record -o /tmp/trace.dat -e sched:sched_switch sleep 2`
   records a short scheduler trace. Copy that actual file to the host before
   selecting it in the offline trace-cmd report form. Captures can contain secrets.
6. Return through Terminal Sessions to the same GDB terminal. `detach` resumes
   the guest and leaves GDB at a prompt; reconnect with `target remote PATH`,
   using the private Unix socket printed at launch. `quit` ends the managed guest.

KASAN and KCSAN require separate resolved kernel configurations and deliberate
rebuild/boot cycles; the configuration-preparation forms are not live detectors.
Likewise, a successful QEMU stub session does not validate physical-board KGDB
or a working kdump crash kernel. Never infer those results from tool presence.

### Preparing a trace-capable Poky image

Use a separate development build/profile, not a production image. For example,
an image recipe based on `core-image-minimal` can add:

```bitbake
IMAGE_FEATURES += "ssh-server-openssh"
IMAGE_INSTALL:append = " strace perf trace-cmd bpftrace lttng-tools lttng-modules"
```

Ensure the configured layers provide each package; `bpftrace` may require
meta-openembedded's `meta-oe` and builds a substantial LLVM dependency. Configure
the guest account/authentication deliberately. An SSH server alone does not
permit passwordless root login.

Resolve the kernel provider's fragments and inspect the resulting `.config`:
ELF/DWARF symbols and retained sources for GDB; tracing/tracepoints and perf
events for perf/trace-cmd; BPF syscall, BPF events and BTF for bpftrace; dynamic
debug and kmemleak for their respective views. LTTng also needs tracer modules
matching the exact running kernel. Tool installation does not enable these
kernel facilities. Lockdep and other instrumentation increase overhead; size
guest memory appropriately and verify `uname -r` and the actual resolved config.

Keep KASAN and KCSAN in separate builds. Config-fragment export is non-destructive
preparation, not a promise that either detector is active in the current guest.

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
