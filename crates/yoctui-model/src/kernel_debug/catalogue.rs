#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelDebugTool {
    GdbRemote,
    GdbCore,
    Strace,
    Perf,
    TraceCmd,
    Ftrace,
    Dmesg,
    DynamicDebug,
    Kmemleak,
    Bpftrace,
    Lttng,
    Crash,
    Kgdb,
    Sanitizers,
    Lockdep,
    SysrqKdump,
    QemuGdb,
    KgdbSerial,
}

impl KernelDebugTool {
    pub const ALL: [Self; 18] = [
        Self::GdbRemote,
        Self::GdbCore,
        Self::Strace,
        Self::Perf,
        Self::TraceCmd,
        Self::Ftrace,
        Self::Dmesg,
        Self::DynamicDebug,
        Self::Kmemleak,
        Self::Bpftrace,
        Self::Lttng,
        Self::Crash,
        Self::Kgdb,
        Self::Sanitizers,
        Self::Lockdep,
        Self::SysrqKdump,
        Self::QemuGdb,
        Self::KgdbSerial,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::GdbRemote => "GDB · remote kernel/QEMU",
            Self::GdbCore => "GDB · userspace core",
            Self::Strace => "strace · syscall attach",
            Self::Perf => "perf · live CPU profile",
            Self::TraceCmd => "trace-cmd · offline report",
            Self::Ftrace => "ftrace · current snapshot",
            Self::Dmesg => "dmesg · kernel log follow",
            Self::DynamicDebug => "dynamic debug · callsite inventory",
            Self::Kmemleak => "kmemleak · existing report",
            Self::Bpftrace => "bpftrace · syscall counts",
            Self::Lttng => "LTTng · kernel-event discovery",
            Self::Crash => "crash · kernel vmcore analysis",
            Self::Kgdb => "KGDB / KDB setup",
            Self::Sanitizers => "KASAN / KCSAN / UBSAN",
            Self::Lockdep => "lockdep / hung tasks",
            Self::SysrqKdump => "SysRq / kdump setup",
            Self::QemuGdb => "QEMU → GDB · managed boot",
            Self::KgdbSerial => "KGDB → GDB · serial board",
        }
    }

    pub const fn program(self) -> Option<&'static str> {
        match self {
            Self::GdbRemote | Self::GdbCore | Self::KgdbSerial => Some("gdb"),
            Self::Strace => Some("strace"),
            Self::Perf => Some("perf"),
            Self::TraceCmd => Some("trace-cmd"),
            Self::Ftrace | Self::DynamicDebug | Self::Kmemleak => Some("cat"),
            Self::Dmesg => Some("dmesg"),
            Self::Bpftrace => Some("bpftrace"),
            Self::Lttng => Some("lttng"),
            Self::Crash => Some("crash"),
            Self::QemuGdb => Some("runqemu"),
            _ => None,
        }
    }

    pub const fn configuration_prep(self) -> bool {
        matches!(self, Self::Sanitizers | Self::Lockdep)
    }

    pub const fn runtime_target(self) -> bool {
        matches!(
            self,
            Self::Strace
                | Self::Perf
                | Self::Ftrace
                | Self::Dmesg
                | Self::DynamicDebug
                | Self::Kmemleak
                | Self::Bpftrace
                | Self::Lttng
        )
    }

    pub const fn guide(self) -> &'static str {
        match self {
            Self::KgdbSerial => {
                "HOST GDB client for an already halted PHYSICAL BOARD. Supply exact running-kernel .config and matching uncompressed vmlinux with DWARF; architecture-capable GDB and exclusive serial access are required. CONFIG_KGDB, CONFIG_KGDB_SERIAL_CONSOLE and CONFIG_DEBUG_INFO must be y for this built-in transport workflow. Select the real host /dev/tty device (not a symlink) and the target UART name separately. Close other console clients. Set readiness to yes ONLY when kgdboc is configured and the target is stopped. Host files cannot prove target readiness or matching build. No config writes, rebuild/deploy/reset, serial break or SysRq is automatic. kgdboc cannot reliably interrupt a running board with GDB Ctrl+C; after continue, re-entry may need manually approved SysRq-G. Use bt, break, continue and deliberate detach; killing the host GDB is not a promise to resume the board. CONFIG_KGDB_KDB is optional for manual KDB; frame pointers aid backtraces, strict RWX may require hardware breakpoints."
            }
            Self::QemuGdb => {
                "Managed Linux host guest, not the physical target. Select exact .qemuboot.conf, boot kernel, rootfs and matching uncompressed vmlinux with DWARF. Direct kernel boot only; flash-only configs are unsupported. Linux, runqemu/native QEMU and GDB 9+ with the guest architecture are required. Snapshot/nonetwork, nokaslr and paused CPUs; private Unix debug socket, separate bounded console log. In GDB: break start_kernel, continue, bt; Ctrl+C interrupts the guest and quit stops it. No rebuild, install or privilege changes. Matching kernel/symbols are your responsibility; ELF presence alone does not establish a match."
            }
            Self::GdbRemote => {
                "Host GDB connects to an already configured QEMU/KGDB TCP stub. Supply matching uncompressed vmlinux with debug symbols and architecture-capable GDB. Breakpoints can stop the whole target. No guest boot or stub setup is automatic. Useful commands: info threads, bt, break, continue. Init files/auto-loaded scripts are disabled; explicitly trust kernel GDB helpers before loading them manually."
            }
            Self::GdbCore => {
                "Offline userspace core analysis, not a kernel vmcore debugger. Supply the exact executable with symbols and its core. Useful commands: bt full, info threads, thread apply all bt. Init files/auto-loaded scripts are disabled. Core dumps can contain secrets."
            }
            Self::Strace => {
                "Userspace syscall tracing, NOT a kernel source debugger. Attach to a PID on the chosen host/target; follows children and displays timestamps/durations. ptrace access is required. Attachment changes timing and may expose arguments/data. Ctrl+C detaches; do not select a critical service without considering disruption."
            }
            Self::Perf => {
                "Live CPU sampling for an explicit PID. Requires perf plus permitted perf_event access, matching kernel support and symbols. Sampling adds overhead. Kernel symbols/stack unwinding can require debug symbols and kernel configuration. This route does not change perf_event_paranoid or elevate privileges."
            }
            Self::TraceCmd => {
                "Offline trace-cmd report of an existing trace.dat; automatic plugins are disabled. Recording is manual: select relevant events and a bounded capture rather than tracing everything. Enable kernel tracing options and install trace-cmd on the capture system. Report files must match the capture and can contain sensitive workload details."
            }
            Self::Ftrace => {
                "Reads /sys/kernel/tracing/trace without clearing or enabling it. Requires mounted tracefs and read permission. Configure tracers/events manually; this route neither mounts tracefs nor writes controls. A snapshot during active tracing may be inconsistent. Function/function_graph and sched/IRQ events help latency analysis."
            }
            Self::Dmesg => {
                "Follow kernel messages without clearing the ring buffer. Requires dmesg with -w support and permission under dmesg_restrict. Diagnose boot, driver, warning, oops and sanitizer reports. Logs can contain sensitive data; access failure stays in terminal output."
            }
            Self::DynamicDebug => {
                "Read /sys/kernel/debug/dynamic_debug/control callsites. Requires dynamic-debug support, debugfs and permission. Enable selected module/file/function callsites manually, then inspect kernel logs. No write, mount or logging change is performed here."
            }
            Self::Kmemleak => {
                "Read the existing /sys/kernel/debug/kmemleak report. Requires CONFIG_DEBUG_KMEMLEAK, debugfs and permissions. Scanning/clearing is manual; suspected leaks need investigation and may be false positives. This route does not trigger a scan or change detector state."
            }
            Self::Bpftrace => {
                "Count one explicit syscall tracepoint by process name; Ctrl+C prints accumulated counts and removes probes. Requires bpftrace, BPF/tracing support and appropriate privilege. Instrumentation adds overhead; tracepoints vary by kernel. No arbitrary BPF program or privilege elevation is accepted."
            }
            Self::Lttng => {
                "List available kernel events, not a recording. Requires LTTng tools and kernel tracer modules/permissions on the selected system. Configure sessions, channels and bounded recording manually; no tracing session is created here."
            }
            Self::Crash => {
                "Offline kernel dump analysis with crash; crashrc startup files are disabled. Supply matching vmlinux/debug symbols and vmcore, architecture-compatible crash and a previously collected kdump. Useful commands: bt, log, ps, kmem. Collection/reboot is not automatic; vmcore may contain secrets."
            }
            Self::Kgdb => {
                "GUIDE ONLY. Enable KGDB and debug information; optionally KDB and KGDB serial-console support. Use matching vmlinux on the debugger host. Configure the target transport/boot arguments intentionally; kgdbwait/breakpoints stop execution. QEMU can provide its own GDB stub. Use GDB remote for an approved TCP stub or KGDB → GDB serial board for an already configured/halted serial target. Transport setup and KDB console commands remain manual."
            }
            Self::Sanitizers => {
                "CONFIG PREP. Inspect an exact kernel .config, review a requested KASAN, KCSAN or UBSAN fragment and explicitly export a NEW .cfg. Config match is file evidence only, not architecture/compiler/runtime support. Integrate using your provider's supported fragment workflow, check the resolved config, then deliberately build/boot and inspect matching dmesg reports. Memory/performance overhead changes; no automatic config/layer edits, build, boot or self-tests."
            }
            Self::Lockdep => {
                "CONFIG PREP. Review a lockdep/atomic-sleep fragment from an exact .config and explicitly export a NEW .cfg. PROVE_LOCKING checks lock ordering; DEBUG_ATOMIC_SLEEP diagnoses sleeping-in-atomic bugs. Hung-task/RCU-stall checks remain manual. Resolve supported settings through your kernel provider, deliberately build/boot, reproduce under controlled load and inspect matching dmesg. Overhead increases; no automatic config edits, boot or locking self-tests."
            }
            Self::SysrqKdump => {
                "GUIDE ONLY. SysRq stack/task dumps aid hang analysis; some SysRq actions crash or reboot the system. Configure crashkernel reservation, kexec/kdump and dump storage deliberately before collecting vmcore. Yoctui never writes sysrq-trigger, crashes/reboots the target or modifies crash settings."
            }
        }
    }

    pub const fn reference(self) -> &'static str {
        match self {
            Self::QemuGdb => "https://www.qemu.org/docs/master/system/gdb.html",
            Self::GdbRemote | Self::Kgdb | Self::KgdbSerial => {
                "https://docs.kernel.org/process/debugging/kgdb.html"
            }
            Self::GdbCore => "https://sourceware.org/gdb/current/onlinedocs/gdb/Files.html",
            Self::Strace => "https://strace.io/",
            Self::Perf => {
                "https://git.kernel.org/pub/scm/linux/kernel/git/torvalds/linux.git/tree/tools/perf/Documentation/perf-top.txt"
            }
            Self::TraceCmd => {
                "https://www.trace-cmd.org/Documentation/trace-cmd/trace-cmd-report.1.html"
            }
            Self::Ftrace => "https://docs.kernel.org/trace/ftrace.html",
            Self::Dmesg => {
                "https://github.com/util-linux/util-linux/blob/master/sys-utils/dmesg.1.adoc"
            }
            Self::DynamicDebug => "https://docs.kernel.org/admin-guide/dynamic-debug-howto.html",
            Self::Kmemleak => "https://docs.kernel.org/dev-tools/kmemleak.html",
            Self::Bpftrace => "https://bpftrace.org/docs/release_024/language",
            Self::Lttng => "https://lttng.org/man/1/lttng-list/v2.15/",
            Self::Crash => "https://crash-utility.github.io/",
            Self::Sanitizers => "https://docs.kernel.org/dev-tools/kasan.html",
            Self::Lockdep => "https://docs.kernel.org/locking/lockdep-design.html",
            Self::SysrqKdump => "https://docs.kernel.org/admin-guide/kdump/kdump.html",
        }
    }
}
